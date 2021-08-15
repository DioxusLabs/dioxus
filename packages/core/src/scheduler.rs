use std::any::Any;

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::pin::Pin;

use futures_util::future::FusedFuture;
use futures_util::pin_mut;
use futures_util::Future;
use futures_util::FutureExt;
use futures_util::StreamExt;
use indexmap::IndexSet;

use crate::innerlude::*;

/// The "Mutations" object holds the changes that need to be made to the DOM.
///
#[derive(Debug)]
pub struct Mutations<'s> {
    pub edits: Vec<DomEdit<'s>>,
    pub noderefs: Vec<NodeRefMutation<'s>>,
}

impl<'s> Mutations<'s> {
    pub fn new() -> Self {
        let edits = Vec::new();
        let noderefs = Vec::new();
        Self { edits, noderefs }
    }

    pub fn extend(&mut self, other: &mut Mutations) {}
}

// refs are only assigned once
pub struct NodeRefMutation<'a> {
    element: &'a mut Option<once_cell::sync::OnceCell<Box<dyn Any>>>,
    element_id: ElementId,
}

impl<'a> std::fmt::Debug for NodeRefMutation<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRefMutation")
            .field("element_id", &self.element_id)
            .finish()
    }
}

impl<'a> NodeRefMutation<'a> {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.element
            .as_ref()
            .and_then(|f| f.get())
            .and_then(|f| f.downcast_ref::<T>())
    }
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.element
            .as_mut()
            .and_then(|f| f.get_mut())
            .and_then(|f| f.downcast_mut::<T>())
    }
}

pub struct Scheduler {
    current_priority: EventPriority,

    pending_events: VecDeque<EventTrigger>,

    pending_immediates: VecDeque<ScopeId>,

    pending_tasks: VecDeque<EventTrigger>,

    garbage_scopes: HashSet<ScopeId>,

    shared: SharedResources,

    high_priorty: PriorityFiber<'static>,
    medium_priority: PriorityFiber<'static>,
    low_priority: PriorityFiber<'static>,
}

pub enum FiberResult<'a> {
    Done(Mutations<'a>),
    Interrupted,
}

impl Scheduler {
    pub fn new(shared: SharedResources) -> Self {
        Self {
            shared,

            // a storage for our receiver to dump into
            pending_events: VecDeque::new(),
            pending_immediates: VecDeque::new(),
            pending_tasks: VecDeque::new(),

            garbage_scopes: HashSet::new(),

            current_priority: EventPriority::Low,

            high_priorty: PriorityFiber::new(),
            medium_priority: PriorityFiber::new(),
            low_priority: PriorityFiber::new(),
        }
    }

    // channels don't have these methods, so we just implement our own wrapper
    pub fn next_event(&mut self) -> Option<EventTrigger> {
        // pop the last event off the internal queue
        self.pending_events.pop_back().or_else(|| {
            self.shared
                .ui_event_receiver
                .borrow_mut()
                .try_next()
                .ok()
                .flatten()
        })
    }

    pub fn manually_poll_events(&mut self) {
        // 1. Poll the UI event receiver
        // 2. Poll the set_state receiver

        // poll the immediates first, creating work out of them
        let shared_receiver = self.shared.immediate_receiver.clone();
        let mut receiver = shared_receiver.borrow_mut();
        while let Ok(Some(trigger)) = receiver.try_next() {
            self.add_dirty_scope(trigger, EventPriority::Low);
        }

        // next poll the UI events,
        let mut receiver = self.shared.ui_event_receiver.borrow_mut();
        while let Ok(Some(trigger)) = receiver.try_next() {
            self.pending_events.push_back(trigger);
        }
    }

    // Converts UI events into dirty scopes with various priorities
    pub fn consume_pending_events(&mut self) -> Result<()> {
        while let Some(trigger) = self.pending_events.pop_back() {
            match &trigger.event {
                VirtualEvent::AsyncEvent { .. } => {}

                // This suspense system works, but it's not the most elegant solution.
                // TODO: Replace this system
                VirtualEvent::SuspenseEvent { hook_idx, domnode } => {
                    todo!("suspense needs to be converted into its own channel");

                    // // Safety: this handler is the only thing that can mutate shared items at this moment in tim
                    // let scope = diff_machine.get_scope_mut(&trigger.originator).unwrap();

                    // // safety: we are sure that there are no other references to the inner content of suspense hooks
                    // let hook = unsafe { scope.hooks.get_mut::<SuspenseHook>(*hook_idx) }.unwrap();

                    // let cx = Context { scope, props: &() };
                    // let scx = SuspendedContext { inner: cx };

                    // // generate the new node!
                    // let nodes: Option<VNode> = (&hook.callback)(scx);

                    // if let Some(nodes) = nodes {
                    //     // allocate inside the finished frame - not the WIP frame
                    //     let nodes = scope.frames.finished_frame().bump.alloc(nodes);

                    //     // push the old node's root onto the stack
                    //     let real_id = domnode.get().ok_or(Error::NotMounted)?;
                    //     diff_machine.edit_push_root(real_id);

                    //     // push these new nodes onto the diff machines stack
                    //     let meta = diff_machine.create_vnode(&*nodes);

                    //     // replace the placeholder with the new nodes we just pushed on the stack
                    //     diff_machine.edit_replace_with(1, meta.added_to_stack);
                    // } else {
                    //     log::warn!(
                    //         "Suspense event came through, but there were no generated nodes >:(."
                    //     );
                    // }
                }

                VirtualEvent::ClipboardEvent(_)
                | VirtualEvent::CompositionEvent(_)
                | VirtualEvent::KeyboardEvent(_)
                | VirtualEvent::FocusEvent(_)
                | VirtualEvent::FormEvent(_)
                | VirtualEvent::SelectionEvent(_)
                | VirtualEvent::TouchEvent(_)
                | VirtualEvent::UIEvent(_)
                | VirtualEvent::WheelEvent(_)
                | VirtualEvent::MediaEvent(_)
                | VirtualEvent::AnimationEvent(_)
                | VirtualEvent::TransitionEvent(_)
                | VirtualEvent::ToggleEvent(_)
                | VirtualEvent::MouseEvent(_)
                | VirtualEvent::PointerEvent(_) => {
                    if let Some(scope) = self.shared.get_scope_mut(trigger.originator) {
                        if let Some(element) = trigger.real_node_id {
                            scope.call_listener(trigger.event, element)?;

                            let receiver = self.shared.immediate_receiver.clone();
                            let mut receiver = receiver.borrow_mut();

                            // Drain the immediates into the dirty scopes, setting the appropiate priorities
                            while let Ok(Some(dirty_scope)) = receiver.try_next() {
                                self.add_dirty_scope(dirty_scope, trigger.priority)
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // nothing to do, no events on channels, no work
    pub fn has_any_work(&self) -> bool {
        self.has_work() || self.has_pending_events() || self.has_pending_garbage()
    }

    pub fn has_pending_events(&self) -> bool {
        self.pending_events.len() > 0
    }

    pub fn has_work(&self) -> bool {
        let has_work = self.high_priorty.has_work()
            || self.medium_priority.has_work()
            || self.low_priority.has_work();
        !has_work
    }

    pub fn has_pending_garbage(&self) -> bool {
        !self.garbage_scopes.is_empty()
    }

    /// If a the fiber finishes its works (IE needs to be committed) the scheduler will drop the dirty scope
    pub fn work_with_deadline(
        &mut self,
        mut deadline: &mut Pin<Box<impl FusedFuture<Output = ()>>>,
        is_deadline_reached: &mut impl FnMut() -> bool,
    ) -> FiberResult {
        // check if we need to elevate priority
        self.current_priority = match (
            self.high_priorty.has_work(),
            self.medium_priority.has_work(),
            self.low_priority.has_work(),
        ) {
            (true, _, _) => EventPriority::High,
            (false, true, _) => EventPriority::Medium,
            (false, false, _) => EventPriority::Low,
        };

        let mut current_fiber = match self.current_priority {
            EventPriority::High => &mut self.high_priorty,
            EventPriority::Medium => &mut self.medium_priority,
            EventPriority::Low => &mut self.low_priority,
        };

        todo!()
    }

    // waits for a trigger, canceling early if the deadline is reached
    // returns true if the deadline was reached
    // does not return the trigger, but caches it in the scheduler
    pub async fn wait_for_any_trigger(
        &mut self,
        mut deadline: &mut Pin<Box<impl FusedFuture<Output = ()>>>,
    ) -> bool {
        use futures_util::select;

        let _immediates = self.shared.immediate_receiver.clone();
        let mut immediates = _immediates.borrow_mut();

        let mut _ui_events = self.shared.ui_event_receiver.clone();
        let mut ui_events = _ui_events.borrow_mut();

        let mut _tasks = self.shared.async_tasks.clone();
        let mut tasks = _tasks.borrow_mut();

        // set_state called
        select! {
            dirty_scope = immediates.next() => {
                if let Some(scope) = dirty_scope {
                    self.add_dirty_scope(scope, EventPriority::Low);
                }
            }

            ui_event = ui_events.next() => {
                if let Some(event) = ui_event {
                    self.pending_events.push_back(event);
                }
            }

            async_task = tasks.next() => {
                if let Some(event) = async_task {
                    self.pending_events.push_back(event);
                }
            }

            _ = deadline => {
                return true;
            }

        }
        false
    }

    pub fn add_dirty_scope(&mut self, scope: ScopeId, priority: EventPriority) {
        match priority {
            EventPriority::High => self.high_priorty.dirty_scopes.insert(scope),
            EventPriority::Medium => self.medium_priority.dirty_scopes.insert(scope),
            EventPriority::Low => self.low_priority.dirty_scopes.insert(scope),
        };
    }

    pub fn clean_up_garbage(&mut self) {
        let mut scopes_to_kill = Vec::new();
        let mut garbage_list = Vec::new();

        for scope in self.garbage_scopes.drain() {
            let scope = self.shared.get_scope_mut(scope).unwrap();
            for node in scope.consume_garbage() {
                garbage_list.push(node);
            }

            while let Some(node) = garbage_list.pop() {
                match &node.kind {
                    VNodeKind::Text(_) => {
                        self.shared.collect_garbage(node.direct_id());
                    }
                    VNodeKind::Anchor(_) => {
                        self.shared.collect_garbage(node.direct_id());
                    }
                    VNodeKind::Suspended(_) => {
                        self.shared.collect_garbage(node.direct_id());
                    }

                    VNodeKind::Element(el) => {
                        self.shared.collect_garbage(node.direct_id());
                        for child in el.children {
                            garbage_list.push(child);
                        }
                    }

                    VNodeKind::Fragment(frag) => {
                        for child in frag.children {
                            garbage_list.push(child);
                        }
                    }

                    VNodeKind::Component(comp) => {
                        // TODO: run the hook destructors and then even delete the scope

                        let scope_id = comp.ass_scope.get().unwrap();
                        let scope = self.shared.get_scope(scope_id).unwrap();
                        let root = scope.root();
                        garbage_list.push(root);
                        scopes_to_kill.push(scope_id);
                    }
                }
            }
        }

        for scope in scopes_to_kill.drain(..) {
            //
            // kill em
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub struct DirtyScope {
    height: u32,
    start_tick: u32,
}

// fibers in dioxus aren't exactly the same as React's. Our fibers are more like a "saved state" of the diffing algorithm.
pub struct PriorityFiber<'a> {
    // scopes that haven't been updated yet
    pending_scopes: Vec<ScopeId>,

    pending_nodes: Vec<*const VNode<'a>>,

    // WIP edits
    edits: Vec<DomEdit<'a>>,

    started: bool,

    // a fiber is finished when no more scopes or nodes are pending
    completed: bool,

    dirty_scopes: IndexSet<ScopeId>,

    wip_edits: Vec<DomEdit<'a>>,

    current_batch_scopes: HashSet<ScopeId>,
}

impl PriorityFiber<'_> {
    fn new() -> Self {
        Self {
            pending_scopes: Vec::new(),
            pending_nodes: Vec::new(),
            edits: Vec::new(),
            started: false,
            completed: false,
            dirty_scopes: IndexSet::new(),
            wip_edits: Vec::new(),
            current_batch_scopes: HashSet::new(),
        }
    }

    fn has_work(&self) -> bool {
        self.dirty_scopes.is_empty()
            && self.wip_edits.is_empty()
            && self.current_batch_scopes.is_empty()
    }
}

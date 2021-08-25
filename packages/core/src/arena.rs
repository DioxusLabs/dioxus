use std::cell::{RefCell, RefMut};
use std::fmt::Display;
use std::{cell::UnsafeCell, rc::Rc};

use crate::heuristics::*;
use crate::innerlude::*;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::stream::FuturesUnordered;
use fxhash::{FxHashMap, FxHashSet};
use slab::Slab;
use smallvec::SmallVec;

use std::any::Any;

use std::any::TypeId;
use std::cell::Ref;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::pin::Pin;

use futures_util::future::FusedFuture;
use futures_util::pin_mut;
use futures_util::Future;
use futures_util::FutureExt;
use futures_util::StreamExt;

type UiReceiver = UnboundedReceiver<EventTrigger>;
type UiSender = UnboundedSender<EventTrigger>;

type TaskReceiver = UnboundedReceiver<ScopeId>;
type TaskSender = UnboundedSender<ScopeId>;

/// These are resources shared among all the components and the virtualdom itself
pub struct Scheduler {
    pub components: UnsafeCell<Slab<Scope>>,

    pub(crate) heuristics: HeuristicsEngine,

    // Used by "set_state" and co - is its own queue
    pub immediate_sender: TaskSender,
    pub immediate_receiver: TaskReceiver,

    /// Triggered by event listeners
    pub ui_event_sender: UiSender,
    pub ui_event_receiver: UiReceiver,

    // Garbage stored
    pub pending_garbage: FxHashSet<ScopeId>,

    // In-flight futures
    pub async_tasks: FuturesUnordered<FiberTask>,

    /// We use a SlotSet to keep track of the keys that are currently being used.
    /// However, we don't store any specific data since the "mirror"
    pub raw_elements: Slab<()>,

    pub task_setter: Rc<dyn Fn(ScopeId)>,

    // scheduler stuff
    pub current_priority: EventPriority,

    pub pending_events: VecDeque<EventTrigger>,

    pub pending_immediates: VecDeque<ScopeId>,

    pub pending_tasks: VecDeque<EventTrigger>,

    pub garbage_scopes: HashSet<ScopeId>,

    pub high_priorty: PriortySystem,
    pub medium_priority: PriortySystem,
    pub low_priority: PriortySystem,
}

impl Scheduler {
    pub fn new() -> Self {
        // preallocate 2000 elements and 20 scopes to avoid dynamic allocation
        let components: UnsafeCell<Slab<Scope>> = UnsafeCell::new(Slab::with_capacity(100));

        // elements are super cheap - the value takes no space
        let raw_elements = Slab::with_capacity(2000);

        let (ui_sender, ui_receiver) = futures_channel::mpsc::unbounded();
        let (immediate_sender, immediate_receiver) = futures_channel::mpsc::unbounded();

        let heuristics = HeuristicsEngine::new();

        // we allocate this task setter once to save us from having to allocate later
        let task_setter = {
            let queue = immediate_sender.clone();
            let components = components.clone();
            Rc::new(move |idx: ScopeId| {
                let comps = unsafe { &*components.get() };

                if let Some(scope) = comps.get(idx.0) {
                    // todo!("implement immediates again")
                    //

                    // queue
                    // .unbounded_send(EventTrigger::new(
                    //     V
                    //     idx,
                    //     None,
                    //     EventPriority::High,
                    // ))
                    // .expect("The event queu receiver should *never* be dropped");
                }
            }) as Rc<dyn Fn(ScopeId)>
        };

        Self {
            components,
            async_tasks: FuturesUnordered::new(),

            ui_event_receiver: ui_receiver,
            ui_event_sender: ui_sender,

            immediate_receiver: immediate_receiver,
            immediate_sender: immediate_sender,

            pending_garbage: FxHashSet::default(),

            heuristics: heuristics,
            raw_elements: raw_elements,
            task_setter,

            // a storage for our receiver to dump into
            pending_events: VecDeque::new(),
            pending_immediates: VecDeque::new(),
            pending_tasks: VecDeque::new(),

            garbage_scopes: HashSet::new(),

            current_priority: EventPriority::Low,

            high_priorty: PriortySystem::new(),
            medium_priority: PriortySystem::new(),
            low_priority: PriortySystem::new(),
        }
    }

    /// this is unsafe because the caller needs to track which other scopes it's already using
    pub fn get_scope(&self, idx: ScopeId) -> Option<&Scope> {
        let inner = unsafe { &*self.components.get() };
        inner.get(idx.0)
    }

    /// this is unsafe because the caller needs to track which other scopes it's already using
    pub fn get_scope_mut(&self, idx: ScopeId) -> Option<&mut Scope> {
        let inner = unsafe { &mut *self.components.get() };
        inner.get_mut(idx.0)
    }

    pub fn with_scope<'b, O: 'static>(
        &'b self,
        _id: ScopeId,
        _f: impl FnOnce(&'b mut Scope) -> O,
    ) -> Result<O> {
        todo!()
    }

    // return a bumpframe with a lifetime attached to the arena borrow
    // this is useful for merging lifetimes
    pub fn with_scope_vnode<'b>(
        &self,
        _id: ScopeId,
        _f: impl FnOnce(&mut Scope) -> &VNode<'b>,
    ) -> Result<&VNode<'b>> {
        todo!()
    }

    pub fn try_remove(&self, id: ScopeId) -> Result<Scope> {
        let inner = unsafe { &mut *self.components.get() };
        Ok(inner.remove(id.0))
        // .try_remove(id.0)
        // .ok_or_else(|| Error::FatalInternal("Scope not found"))
    }

    pub fn reserve_node(&self) -> ElementId {
        ElementId(self.raw_elements.borrow_mut().insert(()))
    }

    /// return the id, freeing the space of the original node
    pub fn collect_garbage(&self, id: ElementId) {
        self.raw_elements.borrow_mut().remove(id.0);
    }

    pub fn insert_scope_with_key(&self, f: impl FnOnce(ScopeId) -> Scope) -> ScopeId {
        let g = unsafe { &mut *self.components.get() };
        let entry = g.vacant_entry();
        let id = ScopeId(entry.key());
        entry.insert(f(id));
        id
    }

    pub fn schedule_update(&self) -> Rc<dyn Fn(ScopeId)> {
        self.task_setter.clone()
    }

    pub fn submit_task(&self, task: FiberTask) -> TaskHandle {
        self.async_tasks.borrow_mut().push(task);
        TaskHandle {}
    }

    pub fn make_trigger_key(&self, trigger: &EventTrigger) -> EventKey {
        let height = self
            .get_scope(trigger.originator)
            .map(|f| f.height)
            .unwrap();

        EventKey {
            height,
            originator: trigger.originator,
            priority: trigger.priority,
        }
    }

    pub fn clean_up_garbage(&mut self) {
        let mut scopes_to_kill = Vec::new();
        let mut garbage_list = Vec::new();

        for scope in self.garbage_scopes.drain() {
            let scope = self.get_scope_mut(scope).unwrap();
            for node in scope.consume_garbage() {
                garbage_list.push(node);
            }

            while let Some(node) = garbage_list.pop() {
                match &node {
                    VNode::Text(_) => {
                        self.collect_garbage(node.direct_id());
                    }
                    VNode::Anchor(_) => {
                        self.collect_garbage(node.direct_id());
                    }
                    VNode::Suspended(_) => {
                        self.collect_garbage(node.direct_id());
                    }

                    VNode::Element(el) => {
                        self.collect_garbage(node.direct_id());
                        for child in el.children {
                            garbage_list.push(child);
                        }
                    }

                    VNode::Fragment(frag) => {
                        for child in frag.children {
                            garbage_list.push(child);
                        }
                    }

                    VNode::Component(comp) => {
                        // TODO: run the hook destructors and then even delete the scope

                        let scope_id = comp.ass_scope.get().unwrap();
                        let scope = self.get_scope(scope_id).unwrap();
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

    // channels don't have these methods, so we just implement our own wrapper
    pub fn next_event(&mut self) -> Option<EventTrigger> {
        // pop the last event off the internal queue
        self.pending_events.pop_back().or_else(|| {
            self.ui_event_receiver
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
        let shared_receiver = self.immediate_receiver.clone();
        let mut receiver = shared_receiver.borrow_mut();
        while let Ok(Some(trigger)) = receiver.try_next() {
            self.add_dirty_scope(trigger, EventPriority::Low);
        }

        // next poll the UI events,
        let mut receiver = self.ui_event_receiver.borrow_mut();
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
                    if let Some(scope) = self.get_scope_mut(trigger.originator) {
                        if let Some(element) = trigger.real_node_id {
                            scope.call_listener(trigger.event, element)?;

                            let receiver = self.immediate_receiver.clone();
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
        self.high_priorty.has_work()
            || self.medium_priority.has_work()
            || self.low_priority.has_work()
    }

    pub fn has_pending_garbage(&self) -> bool {
        !self.garbage_scopes.is_empty()
    }

    fn get_current_fiber<'a>(&'a mut self) -> &mut DiffMachine<'a> {
        let fib = match self.current_priority {
            EventPriority::High => &mut self.high_priorty,
            EventPriority::Medium => &mut self.medium_priority,
            EventPriority::Low => &mut self.low_priority,
        };
        unsafe { std::mem::transmute(fib) }
    }

    /// If a the fiber finishes its works (IE needs to be committed) the scheduler will drop the dirty scope
    pub async fn work_with_deadline<'a>(
        &mut self,
        mutations: &mut Mutations<'_>,
        deadline: &mut Pin<Box<impl FusedFuture<Output = ()>>>,
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

        let mut machine = DiffMachine::new(mutations, ScopeId(0), &self);

        let dirty_root = {
            let dirty_roots = match self.current_priority {
                EventPriority::High => &self.high_priorty.dirty_scopes,
                EventPriority::Medium => &self.medium_priority.dirty_scopes,
                EventPriority::Low => &self.low_priority.dirty_scopes,
            };
            let mut height = 0;
            let mut dirty_root = {
                let root = dirty_roots.iter().next();
                if root.is_none() {
                    return FiberResult::Done;
                }
                root.unwrap()
            };

            for root in dirty_roots {
                if let Some(scope) = self.get_scope(*root) {
                    if scope.height < height {
                        height = scope.height;
                        dirty_root = root;
                    }
                }
            }
            dirty_root
        };

        match {
            let fut = machine.diff_scope(*dirty_root).fuse();
            pin_mut!(fut);

            match futures_util::future::select(deadline, fut).await {
                futures_util::future::Either::Left((deadline, work_fut)) => true,
                futures_util::future::Either::Right((_, deadline_fut)) => false,
            }
        } {
            true => FiberResult::Done,
            false => FiberResult::Interrupted,
        }
    }

    // waits for a trigger, canceling early if the deadline is reached
    // returns true if the deadline was reached
    // does not return the trigger, but caches it in the scheduler
    pub async fn wait_for_any_trigger(
        &mut self,
        mut deadline: &mut Pin<Box<impl FusedFuture<Output = ()>>>,
    ) -> bool {
        use futures_util::select;

        let _immediates = self.immediate_receiver.clone();
        let mut immediates = _immediates.borrow_mut();

        let mut _ui_events = self.ui_event_receiver.clone();
        let mut ui_events = _ui_events.borrow_mut();

        let mut _tasks = self.async_tasks.clone();
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
}

pub struct TaskHandle {}

impl TaskHandle {
    pub fn toggle(&self) {}
    pub fn start(&self) {}
    pub fn stop(&self) {}
    pub fn restart(&self) {}
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub struct DirtyScope {
    height: u32,
    start_tick: u32,
}

pub struct PriortySystem {
    pub pending_scopes: Vec<ScopeId>,
    pub dirty_scopes: HashSet<ScopeId>,
}

impl PriortySystem {
    pub fn new() -> Self {
        Self {
            pending_scopes: Default::default(),
            dirty_scopes: Default::default(),
        }
    }

    fn has_work(&self) -> bool {
        self.pending_scopes.len() > 0 || self.dirty_scopes.len() > 0
    }
}

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ScopeId(pub usize);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ElementId(pub usize);
impl Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ElementId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

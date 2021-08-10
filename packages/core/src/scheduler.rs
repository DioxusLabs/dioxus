use std::any::Any;

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::pin::Pin;

use futures_util::Future;

use crate::innerlude::*;

/// The "Mutations" object holds the changes that need to be made to the DOM.
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
}

// refs are only assigned once
pub struct NodeRefMutation<'a> {
    element: &'a mut Option<once_cell::sync::OnceCell<Box<dyn Any>>>,
    element_id: ElementId,
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
    pending_immediates: VecDeque<EventTrigger>,
    pending_tasks: VecDeque<EventTrigger>,

    garbage_scopes: HashSet<ScopeId>,

    shared: SharedResources,

    dirty_scopes: [HashSet<DirtyScope>; 3],

    fibers: Vec<Fiber<'static>>,
}

impl Scheduler {
    pub fn new(shared: SharedResources) -> Self {
        Self {
            shared,

            // a storage for our receiver to dump into
            pending_events: VecDeque::new(),
            pending_immediates: VecDeque::new(),
            pending_tasks: VecDeque::new(),

            fibers: Vec::new(),

            garbage_scopes: HashSet::new(),

            current_priority: EventPriority::Low,

            // low, medium, high
            dirty_scopes: [HashSet::new(), HashSet::new(), HashSet::new()],
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
                .and_then(|f| f)
        })
    }

    // waits for a trigger, canceling early if the deadline is reached
    // returns true if the deadline was reached
    pub async fn wait_for_any_trigger(&mut self, deadline: &mut impl Future<Output = ()>) -> bool {
        todo!()
        // match raw_trigger {
        //     Ok(Some(trigger)) => trigger,
        //     _ => {
        //         // nothing to do - let's clean up any garbage we might have
        //         self.scheduler.clean_up_garbage();

        //         // Continuously poll the future pool and the event receiver for work
        //         let mut tasks = self.shared.async_tasks.borrow_mut();
        //         let tasks_tasks = tasks.next();

        //         // if the new event generates work more important than our current fiber, we should consider switching
        //         // only switch if it impacts different scopes.
        //         let mut ui_receiver = self.shared.ui_event_receiver.borrow_mut();
        //         let ui_reciv_task = ui_receiver.next();

        //         // right now, this polling method will only catch batched set_states that don't get awaited.
        //         // However, in the future, we might be interested in batching set_states across await points
        //         let immediate_tasks = ();

        //         futures_util::pin_mut!(tasks_tasks);
        //         futures_util::pin_mut!(ui_reciv_task);

        //         // Poll the event receiver and the future pool for work
        //         // Abort early if our deadline has ran out
        //         let mut deadline = (&mut deadline_future).fuse();

        //         let trig = futures_util::select! {
        //             trigger = tasks_tasks => trigger,
        //             trigger = ui_reciv_task => trigger,

        //             // abort if we're out of time
        //             _ = deadline => { return Ok(Mutations::new()); }
        //             // _ = deadline => { return Ok(diff_machine.mutations); }
        //         };

        //         trig.unwrap()
        //     }
        // };
        //
    }

    pub fn add_dirty_scope(&mut self, scope: ScopeId, priority: EventPriority) {
        //

        // generated_immediates
        //     .entry(dirty_scope)
        //     .and_modify(|cur_priority| {
        //         if *cur_priority > new_priority {
        //             *cur_priority = new_priority;
        //         }
        //     })
        //     .or_insert_with(|| new_priority);
    }

    pub fn has_work(&self) -> bool {
        true
    }

    // returns true if the deadline is reached
    // returns false if the deadline is not reached
    pub fn progress_work(
        &mut self,
        machine: &mut DiffMachine,
        is_deadline_reached: &mut impl FnMut() -> bool,
    ) -> bool {
        todo!()
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
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct DirtyScope {
    height: u32,
    priority: EventPriority,
    start_tick: u32,
}

// fibers in dioxus aren't exactly the same as React's. Our fibers are more like a "saved state" of the diffing algorithm.
pub struct Fiber<'a> {
    // scopes that haven't been updated yet
    pending_scopes: Vec<ScopeId>,

    pending_nodes: Vec<*const VNode<'a>>,

    // WIP edits
    edits: Vec<DomEdit<'a>>,

    started: bool,

    // a fiber is finished when no more scopes or nodes are pending
    completed: bool,
}

impl Fiber<'_> {
    fn new() -> Self {
        Self {
            pending_scopes: Vec::new(),
            pending_nodes: Vec::new(),
            edits: Vec::new(),
            started: false,
            completed: false,
        }
    }
}

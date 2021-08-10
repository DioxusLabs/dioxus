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
                .flatten()
        })
    }

    pub fn manually_poll_channels(&mut self) {
        todo!()
    }

    pub fn consume_pending_events(&mut self) {}

    // nothing to do, no events on channels, no work
    pub fn is_idle(&self) -> bool {
        !self.has_work() && self.has_pending_events()
    }

    pub fn has_pending_events(&self) -> bool {
        todo!()
    }

    pub fn has_work(&self) -> bool {
        todo!()
    }

    pub fn work_with_deadline(
        &mut self,
        deadline: &mut impl Future<Output = ()>,
        is_deadline_reached: &mut impl FnMut() -> bool,
    ) -> FiberResult {
        todo!()
    }

    // waits for a trigger, canceling early if the deadline is reached
    // returns true if the deadline was reached
    // does not return the trigger, but caches it in the scheduler
    pub async fn wait_for_any_trigger(&mut self, deadline: &mut impl Future<Output = ()>) -> bool {
        // poll_all_channels()
        // if no events and no work {
        //     wait for events, cache them
        // }

        // for event in pending_evenst {
        //     mark_dirty_scopes_with_priorities
        // }

        // // level shift the priority
        // match current_priority {
        //     high -> finish high queue
        //     medium -> if work in high { set_current_level_high }
        //     low -> if work in high { set_current_level_high } else if work in medium { set_current_level_medium }
        // }

        // let mut fiber = load_priority_fiber();

        // fiber.load_work(high_priority_work);

        // fiber.progress_with_deadline(deadline);

        // if fiber.finished() {
        //     if work_pending_for_priority(fiber.priority()) {
        //         fiber.load_work(priority_work)
        //     } else {
        //         shift_level_down()
        //     }
        // }

        // load current events
        // if work queue is empty {
        //     wait for events
        //     load current events
        // }

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

/*
Strategy:
1. Check if there are any UI events in the receiver.
2. If there are, run the listener and then mark the dirty nodes
3. If there are dirty nodes to be progressed, do so.
4. Poll the task queue to see if we can create more dirty scopes.
5. Resume any current in-flight work if there is some.
6. While the deadline is not met, progress work, periodically checking the deadline.


How to choose work:
- When a scope is marked as dirty, it is given a priority.
- If a dirty scope chains (borrowed) into children, mark those as dirty as well.
- When the work loop starts, work on the highest priority scopes first.
- Work by priority, choosing to pause in-flight work if higher-priority work is ready.



4. If there are no fibers, then wait for the next event from the receiver. Abort if the deadline is reached.
5. While processing a fiber, periodically check if we're out of time
6. If our deadling is reached, then commit our edits to the realdom
7. Whenever a fiber is finished, immediately commit it. (IE so deadlines can be infinite if unsupported)


// 1. Check if there are any events in the receiver.
// 2. If there are, process them and create a new fiber.
// 3. If there are no events, then choose a fiber to work on.
// 4. If there are no fibers, then wait for the next event from the receiver. Abort if the deadline is reached.
// 5. While processing a fiber, periodically check if we're out of time
// 6. If our deadling is reached, then commit our edits to the realdom
// 7. Whenever a fiber is finished, immediately commit it. (IE so deadlines can be infinite if unsupported)

We slice fibers based on time. Each batch of events between frames is its own fiber. This is the simplest way
to conceptualize what *is* or *isn't* a fiber. IE if a bunch of events occur during a time slice, they all
get batched together as a single operation of "dirty" scopes.

This approach is designed around the "diff during rIC and commit during rAF"

We need to make sure to not call multiple events while the diff machine is borrowing the same scope. Because props
and listeners hold references to hook data, it is wrong to run a scope that is already being diffed.
*/

// // 1. Drain the existing immediates.
// //
// // These are generated by async tasks that we never got a chance to finish.
// // All of these get scheduled with the lowest priority.
// while let Ok(Some(dirty_scope)) = self.shared.immediate_receiver.borrow_mut().try_next()
// {
//     self.scheduler
//         .add_dirty_scope(dirty_scope, EventPriority::Low);
// }

// // 2. Drain the event queues, calling whatever listeners need to be called
// //
// // First, check if there's any set_states in the queue that we can mark as low priority
// // Next, check the UI event receiver so we can mark those scopes as medium/high priority
// // Next, work on any fibers.
// // Once the fiber work is finished
// while let Some(trigger) = self.scheduler.next_event() {
//     match &trigger.event {
//         VirtualEvent::AsyncEvent { .. } => {}

//         // This suspense system works, but it's not the most elegant solution.
//         // TODO: Replace this system
//         VirtualEvent::SuspenseEvent { hook_idx, domnode } => {
//             todo!();
//             // // Safety: this handler is the only thing that can mutate shared items at this moment in tim
//             // let scope = diff_machine.get_scope_mut(&trigger.originator).unwrap();

//             // // safety: we are sure that there are no other references to the inner content of suspense hooks
//             // let hook = unsafe { scope.hooks.get_mut::<SuspenseHook>(*hook_idx) }.unwrap();

//             // let cx = Context { scope, props: &() };
//             // let scx = SuspendedContext { inner: cx };

//             // // generate the new node!
//             // let nodes: Option<VNode> = (&hook.callback)(scx);

//             // if let Some(nodes) = nodes {
//             //     // allocate inside the finished frame - not the WIP frame
//             //     let nodes = scope.frames.finished_frame().bump.alloc(nodes);

//             //     // push the old node's root onto the stack
//             //     let real_id = domnode.get().ok_or(Error::NotMounted)?;
//             //     diff_machine.edit_push_root(real_id);

//             //     // push these new nodes onto the diff machines stack
//             //     let meta = diff_machine.create_vnode(&*nodes);

//             //     // replace the placeholder with the new nodes we just pushed on the stack
//             //     diff_machine.edit_replace_with(1, meta.added_to_stack);
//             // } else {
//             //     log::warn!(
//             //         "Suspense event came through, but there were no generated nodes >:(."
//             //     );
//             // }
//         }

//         VirtualEvent::ClipboardEvent(_)
//         | VirtualEvent::CompositionEvent(_)
//         | VirtualEvent::KeyboardEvent(_)
//         | VirtualEvent::FocusEvent(_)
//         | VirtualEvent::FormEvent(_)
//         | VirtualEvent::SelectionEvent(_)
//         | VirtualEvent::TouchEvent(_)
//         | VirtualEvent::UIEvent(_)
//         | VirtualEvent::WheelEvent(_)
//         | VirtualEvent::MediaEvent(_)
//         | VirtualEvent::AnimationEvent(_)
//         | VirtualEvent::TransitionEvent(_)
//         | VirtualEvent::ToggleEvent(_)
//         | VirtualEvent::MouseEvent(_)
//         | VirtualEvent::PointerEvent(_) => {
//             if let Some(scope) = self.shared.get_scope_mut(trigger.originator) {
//                 if let Some(element) = trigger.real_node_id {
//                     scope.call_listener(trigger.event, element)?;

//                     // Drain the immediates into the dirty scopes, setting the appropiate priorities
//                     while let Ok(Some(dirty_scope)) =
//                         self.shared.immediate_receiver.borrow_mut().try_next()
//                     {
//                         self.scheduler
//                             .add_dirty_scope(dirty_scope, trigger.priority)
//                     }
//                 }
//             }
//         }
//     }
// }

// // todo: for these, make these methods return results where deadlinereached is an error type

// // 3. Work through the fibers, and wait for any future work to be ready
// let mut machine = DiffMachine::new_headless(&self.shared);
// if self.scheduler.progress_work(&mut machine, &mut is_ready) {
//     break;
// }

// // 4. Wait for any new triggers before looping again
// if self
//     .scheduler
//     .wait_for_any_trigger(&mut deadline_future)
//     .await
// {
//     break;
// }

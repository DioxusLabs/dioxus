use futures_channel::mpsc::UnboundedSender;
use futures_util::Future;
use std::{cell::RefCell, rc::Rc};

use crate::innerlude::ScopeId;
/// The type of message that can be sent to the scheduler.
///
/// These messages control how the scheduler will process updates to the UI.
#[derive(Debug)]
pub enum SchedulerMsg {
    /// Events from athe Renderer
    Event,

    /// Immediate updates from Components that mark them as dirty
    Immediate(ScopeId),

    /// Mark all components as dirty and update them
    DirtyAll,

    /// New tasks from components that should be polled when the next poll is ready
    NewTask(ScopeId),
}

// todo extract this so spawning doesn't require refcell and rc doesnt need to be tracked
#[derive(Clone)]
pub struct FutureQueue {
    pub sender: UnboundedSender<SchedulerMsg>,
    pub queue: Rc<RefCell<Vec<Box<dyn Future<Output = ()>>>>>,
}

impl FutureQueue {
    pub fn new(sender: UnboundedSender<SchedulerMsg>) -> Self {
        Self {
            sender,
            queue: Default::default(),
        }
    }

    pub fn spawn(&self, id: ScopeId, fut: impl Future<Output = ()> + 'static) -> TaskId {
        self.sender
            .unbounded_send(SchedulerMsg::NewTask(id))
            .unwrap();
        self.queue.borrow_mut().push(Box::new(fut));

        todo!()
    }

    pub fn remove(&self, id: TaskId) {
        todo!()
    }
}

/// A task's unique identifier.
///
/// `TaskId` is a `usize` that is unique across the entire [`VirtualDom`] and across time. [`TaskID`]s will never be reused
/// once a Task has been completed.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TaskId {
    /// The global ID of the task
    pub id: usize,

    /// The original scope that this task was scheduled in
    pub scope: ScopeId,
}

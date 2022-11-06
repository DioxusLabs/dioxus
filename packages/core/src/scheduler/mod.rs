use std::sync::Arc;

use crate::ScopeId;

mod handle;
mod suspense;
mod task;
mod wait;
mod waker;

pub use handle::*;
use slab::Slab;
pub use suspense::*;
pub use task::*;

/// The type of message that can be sent to the scheduler.
///
/// These messages control how the scheduler will process updates to the UI.
#[derive(Debug)]
pub enum SchedulerMsg {
    /// Events from the Renderer
    Event,

    /// Immediate updates from Components that mark them as dirty
    Immediate(ScopeId),

    /// Mark all components as dirty and update them
    DirtyAll,

    /// A task has woken and needs to be progressed
    TaskNotified(TaskId),

    /// A task has woken and needs to be progressed
    SuspenseNotified(SuspenseId),
}

pub struct Scheduler {
    rx: futures_channel::mpsc::UnboundedReceiver<SchedulerMsg>,
    ready_suspense: Vec<ScopeId>,
    pub handle: SchedulerHandle,
}

impl Scheduler {
    pub fn new() -> Self {
        let (tx, rx) = futures_channel::mpsc::unbounded();
        Self {
            rx,
            handle: SchedulerHandle::new(tx),
            ready_suspense: Default::default(),
        }
    }

    /// Waits for a future to complete that marks the virtualdom as dirty
    ///
    /// Not all messages will mark a virtualdom as dirty, so this waits for a message that has side-effects that do
    pub fn wait_for_work(&mut self) {
        //
    }
}

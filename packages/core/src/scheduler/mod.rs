use crate::ScopeId;
use slab::Slab;

mod suspense;
mod task;
mod wait;
mod waker;

pub use suspense::*;
pub use task::*;
pub use waker::RcWake;

/// The type of message that can be sent to the scheduler.
///
/// These messages control how the scheduler will process updates to the UI.
#[derive(Debug)]
pub(crate) enum SchedulerMsg {
    /// Immediate updates from Components that mark them as dirty
    Immediate(ScopeId),

    /// A task has woken and needs to be progressed
    TaskNotified(TaskId),

    /// A task has woken and needs to be progressed
    SuspenseNotified(SuspenseId),
}

use std::{cell::RefCell, rc::Rc};

pub(crate) struct Scheduler {
    pub sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,

    /// Tasks created with cx.spawn
    pub tasks: RefCell<Slab<Rc<LocalTask>>>,

    /// Async components
    pub leaves: RefCell<Slab<Rc<SuspenseLeaf>>>,
}

impl Scheduler {
    pub fn new(sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>) -> Rc<Self> {
        Rc::new(Scheduler {
            sender,
            tasks: RefCell::new(Slab::new()),
            leaves: RefCell::new(Slab::new()),
        })
    }
}

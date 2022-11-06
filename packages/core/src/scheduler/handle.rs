use futures_util::Future;
use slab::Slab;
use std::{cell::RefCell, pin::Pin, rc::Rc, sync::Arc};

use super::{LocalTask, SchedulerMsg, SuspenseLeaf};

#[derive(Clone)]
pub struct SchedulerHandle(Rc<HandleInner>);

impl std::ops::Deref for SchedulerHandle {
    type Target = HandleInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct HandleInner {
    pub sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,

    /// Tasks created with cx.spawn
    pub tasks: RefCell<Slab<Rc<LocalTask>>>,

    /// Async components
    pub leaves: RefCell<Slab<Rc<SuspenseLeaf>>>,
}

impl SchedulerHandle {
    pub fn new(sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>) -> Self {
        Self(Rc::new(HandleInner {
            sender,
            tasks: RefCell::new(Slab::new()),
            leaves: RefCell::new(Slab::new()),
        }))
    }
}

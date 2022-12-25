use super::{waker::ArcWake, SchedulerMsg};
use crate::ElementId;
use crate::{innerlude::Mutations, Element, ScopeId};
use std::future::Future;
use std::sync::Arc;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
};

/// An ID representing an ongoing suspended component
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct SuspenseId(pub usize);

/// A boundary in the VirtualDom that captures all suspended components below it
pub struct SuspenseContext {
    pub(crate) id: ScopeId,
    pub(crate) waiting_on: RefCell<HashSet<SuspenseId>>,
    pub(crate) mutations: RefCell<Mutations<'static>>,
    pub(crate) placeholder: Cell<Option<ElementId>>,
    pub(crate) created_on_stack: Cell<usize>,
}

impl SuspenseContext {
    /// Create a new boundary for suspense
    pub fn new(id: ScopeId) -> Self {
        Self {
            id,
            waiting_on: Default::default(),
            mutations: RefCell::new(Mutations::default()),
            placeholder: Cell::new(None),
            created_on_stack: Cell::new(0),
        }
    }
}

pub(crate) struct SuspenseLeaf {
    pub(crate) id: SuspenseId,
    pub(crate) scope_id: ScopeId,
    pub(crate) tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
    pub(crate) notified: Cell<bool>,
    pub(crate) task: *mut dyn Future<Output = Element<'static>>,
}

impl ArcWake for SuspenseLeaf {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.notified.set(true);
        _ = arc_self
            .tx
            .unbounded_send(SchedulerMsg::SuspenseNotified(arc_self.id));
    }
}

use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    pin::Pin,
    rc::Rc,
};

use super::{waker::RcWake, SchedulerMsg};
use crate::{innerlude::Mutation, Element, ScopeId};
use futures_task::Waker;
use futures_util::Future;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SuspenseId(pub usize);

pub type SuspenseContext = Rc<RefCell<SuspenseBoundary>>;
/// Essentially a fiber in React
pub struct SuspenseBoundary {
    pub id: ScopeId,
    pub waiting_on: HashSet<SuspenseId>,
    pub mutations: Vec<Mutation<'static>>,
}

impl SuspenseBoundary {
    pub fn new(id: ScopeId) -> Self {
        Self {
            id,
            waiting_on: Default::default(),
            mutations: Default::default(),
        }
    }
}

pub struct SuspenseLeaf {
    pub id: SuspenseId,
    pub scope_id: ScopeId,
    pub boundary: ScopeId,
    pub tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
    pub notified: Cell<bool>,

    pub task: *mut dyn Future<Output = Element<'static>>,
}

impl RcWake for SuspenseLeaf {
    fn wake_by_ref(arc_self: &Rc<Self>) {
        if arc_self.notified.get() {
            return;
        }
        arc_self.notified.set(true);
        _ = arc_self
            .tx
            .unbounded_send(SchedulerMsg::SuspenseNotified(arc_self.id));
    }
}

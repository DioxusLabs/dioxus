use std::{collections::HashSet, rc::Rc};

use futures_task::{RawWaker, RawWakerVTable, Waker};
use futures_util::Future;

use crate::{innerlude::Mutation, Element, Scope, ScopeId};

use super::SchedulerMsg;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SuspenseId(pub usize);

pub type SuspenseContext = Rc<SuspenseBoundary>;
/// Essentially a fiber in React
pub struct SuspenseBoundary {
    pub id: ScopeId,
    waiting_on: HashSet<SuspenseId>,
    mutations: Vec<Mutation<'static>>,
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

/*


many times the future will be ready every time it's polled, so we can spin on it until it doesnt wake us up immediately


*/
pub struct SuspenseLeaf {
    pub id: SuspenseId,
    pub scope: ScopeId,
    pub boundary: ScopeId,
    pub tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,

    pub task: *mut dyn Future<Output = Element<'static>>,
}

pub fn make_suspense_waker(task: &SuspenseLeaf) -> Waker {
    let raw = RawWaker::new(task as *const SuspenseLeaf as *const _, task_vtable());
    unsafe { Waker::from_raw(raw) }
}

fn task_vtable() -> &'static RawWakerVTable {
    &RawWakerVTable::new(clone, wake, wake_by_ref, drop_task)
}

unsafe fn clone(data: *const ()) -> RawWaker {
    RawWaker::new(data as *const (), task_vtable())
}
unsafe fn wake(data: *const ()) {
    wake_by_ref(data);
}
unsafe fn wake_by_ref(data: *const ()) {
    let task = &*(data as *const SuspenseLeaf);
    task.tx
        .unbounded_send(SchedulerMsg::SuspenseNotified(task.id))
        .expect("Scheduler should exist");
}

unsafe fn drop_task(_data: *const ()) {
    // doesnt do anything
}

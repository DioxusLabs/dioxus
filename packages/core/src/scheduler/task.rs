use std::{cell::RefCell, mem, ops::DerefMut, pin::Pin, process::Output, rc::Rc, sync::Arc};

use futures_task::{waker, ArcWake, Context, RawWaker, RawWakerVTable, Waker};
use futures_util::{pin_mut, Future, FutureExt};
use slab::Slab;

use crate::ScopeId;

use super::{HandleInner, SchedulerHandle, SchedulerMsg};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TaskId(pub usize);

/// the task itself is the waker

#[derive(Clone)]
pub struct LocalTask {
    id: TaskId,
    scope: ScopeId,
    tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
    pub task: *mut dyn Future<Output = ()>,
}

impl HandleInner {
    pub fn spawn(&self, scope: ScopeId, task: impl Future<Output = ()> + 'static) -> TaskId {
        let mut tasks = self.tasks.borrow_mut();

        let entry = tasks.vacant_entry();
        let task_id = TaskId(entry.key());

        entry.insert(LocalTask {
            id: task_id,
            tx: self.sender.clone(),
            task: Box::into_raw(Box::new(task)),
            scope,
        });

        self.sender
            .unbounded_send(SchedulerMsg::TaskNotified(task_id))
            .expect("Scheduler should exist");

        task_id
    }

    // drops the future
    pub fn remove(&self, id: TaskId) {
        //
    }

    // Aborts the future
    pub fn abort(&self, id: TaskId) {
        //
    }
}

pub fn make_task_waker(task: &LocalTask) -> Waker {
    let raw = RawWaker::new(task as *const LocalTask as *const _, task_vtable());
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
    let task = &*(data as *const LocalTask);
    task.tx
        .unbounded_send(SchedulerMsg::TaskNotified(task.id))
        .expect("Scheduler should exist");
}

unsafe fn drop_task(_data: *const ()) {
    // doesnt do anything
}

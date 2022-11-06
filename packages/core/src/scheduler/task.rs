use std::{
    cell::{RefCell, UnsafeCell},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::DerefMut,
    pin::Pin,
    process::Output,
    rc::Rc,
    sync::Arc,
};

use futures_task::{waker, ArcWake, Context, RawWaker, RawWakerVTable, Waker};
use futures_util::{pin_mut, Future, FutureExt};
use slab::Slab;

use crate::{Element, ScopeId};

use super::{waker::RcWake, HandleInner, SchedulerHandle, SchedulerMsg};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TaskId(pub usize);

/// the task itself is the waker

pub struct LocalTask {
    id: TaskId,
    scope: ScopeId,
    tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
    pub task: UnsafeCell<Pin<Box<dyn Future<Output = ()> + 'static>>>,
}

impl HandleInner {
    pub fn spawn(&self, scope: ScopeId, task: impl Future<Output = ()> + 'static) -> TaskId {
        let mut tasks = self.tasks.borrow_mut();
        let entry = tasks.vacant_entry();
        let task_id = TaskId(entry.key());

        entry.insert(Rc::new(LocalTask {
            id: task_id,
            tx: self.sender.clone(),
            task: UnsafeCell::new(Box::pin(task)),
            scope,
        }));

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

pub fn make_task_waker(task: Rc<LocalTask>) -> Waker {
    let ptr = Rc::into_raw(task).cast::<()>();
    super::waker::make_rc_waker(task)
}

impl RcWake for LocalTask {
    fn wake_by_ref(arc_self: &Rc<Self>) {
        _ = arc_self
            .tx
            .unbounded_send(SchedulerMsg::TaskNotified(arc_self.id));
    }
}

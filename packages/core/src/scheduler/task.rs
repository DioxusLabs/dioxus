use std::{
    cell::{RefCell, UnsafeCell},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::DerefMut,
    pin::Pin,
    process::Output,
    rc::Rc,
    sync::Arc,
    task::Poll,
};

use futures_util::{pin_mut, Future, FutureExt};
use slab::Slab;
use std::task::{Context, RawWaker, RawWakerVTable, Waker};

use crate::{Element, ScopeId};

use super::{waker::RcWake, HandleInner, Scheduler, SchedulerMsg};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TaskId(pub usize);

/// the task itself is the waker

pub struct LocalTask {
    pub id: TaskId,
    pub scope: ScopeId,
    pub tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,

    // todo: use rc and weak, or the bump slab instead of unsafecell
    pub task: UnsafeCell<Pin<Box<dyn Future<Output = ()> + 'static>>>,
}

impl LocalTask {
    pub fn progress(self: &Rc<Self>) -> bool {
        let waker = self.waker();
        let mut cx = Context::from_waker(&waker);

        // safety: the waker owns its task and everythig is single threaded
        let fut = unsafe { &mut *self.task.get() };

        match fut.poll_unpin(&mut cx) {
            Poll::Ready(_) => true,
            _ => false,
        }
    }
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

impl RcWake for LocalTask {
    fn wake_by_ref(arc_self: &Rc<Self>) {
        _ = arc_self
            .tx
            .unbounded_send(SchedulerMsg::TaskNotified(arc_self.id));
    }
}

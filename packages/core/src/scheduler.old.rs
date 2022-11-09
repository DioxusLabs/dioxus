use std::{cell::RefCell, rc::Rc, sync::Arc};

use futures_task::ArcWake;
use slab::Slab;
use std::future::Future;

use crate::{innerlude::Mutation, ScopeId};

type Shared<T> = Rc<RefCell<T>>;
struct LocalTask {}

pub struct Fiber {
    // The work-in progress of this suspended tree
    pub mutations: Vec<Mutation<'static>>,
}

#[derive(Clone)]
pub struct SchedulerHandle {
    tasks: Shared<Slab<LocalTask>>,
    suspended: Shared<Slab<LocalTask>>,
    fibers: Shared<Slab<Fiber>>,
    tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
}

struct TaskEntry {}

struct LocalTaskWaker<T> {
    future: T,
    id: TaskId,
    tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
}

unsafe impl<T> Send for LocalTaskWaker<T> {}
unsafe impl<T> Sync for LocalTaskWaker<T> {}

impl<T> ArcWake for LocalTaskWaker<T> {
    fn wake(self: Arc<Self>) {
        Self::wake_by_ref(&self)
    }
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self
            .tx
            .unbounded_send(SchedulerMsg::TaskNotified(arc_self.id))
            .unwrap();
    }
}

impl SchedulerHandle {
    fn spawn(&self, fut: impl Future<Output = ()> + 'static) -> TaskId {
        use futures_task::waker;

        let tasks = self.tasks.borrow_mut();
        let entry = tasks.vacant_entry();
        let id = TaskId(entry.key());

        let task = Arc::new(LocalTaskWaker {
            future: fut,
            id,
            tx: self.tx.clone(),
        });

        let local_task = waker(task.clone());

        entry.insert(val);

        //
        todo!()
    }

    fn remove(&self, id: TaskId) {
        //
    }
}

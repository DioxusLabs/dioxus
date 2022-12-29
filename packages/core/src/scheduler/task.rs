use super::{waker::ArcWake, Scheduler, SchedulerMsg};
use crate::ScopeId;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A task's unique identifier.
///
/// `TaskId` is a `usize` that is unique across the entire VirtualDOM and across time. TaskIDs will never be reused
/// once a Task has been completed.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TaskId(pub usize);

/// the task itself is the waker
pub(crate) struct LocalTask {
    pub scope: ScopeId,
    pub(super) task: RefCell<Pin<Box<dyn Future<Output = ()> + 'static>>>,
    id: TaskId,
    tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
}

impl Scheduler {
    /// Start a new future on the same thread as the rest of the VirtualDom.
    ///
    /// This future will not contribute to suspense resolving, so you should primarily use this for reacting to changes
    /// and long running tasks.
    ///
    /// Whenever the component that owns this future is dropped, the future will be dropped as well.
    ///
    /// Spawning a future onto the root scope will cause it to be dropped when the root component is dropped - which
    /// will only occur when the VirtuaalDom itself has been dropped.
    pub fn spawn(&self, scope: ScopeId, task: impl Future<Output = ()> + 'static) -> TaskId {
        let mut tasks = self.tasks.borrow_mut();
        let entry = tasks.vacant_entry();
        let task_id = TaskId(entry.key());

        entry.insert(Arc::new(LocalTask {
            id: task_id,
            tx: self.sender.clone(),
            task: RefCell::new(Box::pin(task)),
            scope,
        }));

        self.sender
            .unbounded_send(SchedulerMsg::TaskNotified(task_id))
            .expect("Scheduler should exist");

        task_id
    }

    /// Drop the future with the given TaskId
    ///
    /// This does nto abort the task, so you'll want to wrap it in an aborthandle if that's important to you
    pub fn remove(&self, id: TaskId) {
        self.tasks.borrow_mut().remove(id.0);
    }
}

impl ArcWake for LocalTask {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        _ = arc_self
            .tx
            .unbounded_send(SchedulerMsg::TaskNotified(arc_self.id));
    }
}

use crate::innerlude::{remove_future, spawn, Runtime};
use crate::ScopeId;
use futures_util::task::ArcWake;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Waker;

/// A task's unique identifier.
///
/// `Task` is a unique identifier for a task that has been spawned onto the runtime. It can be used to cancel the task
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Task(pub(crate) usize);

impl Task {
    /// Start a new future on the same thread as the rest of the VirtualDom.
    ///
    /// This future will not contribute to suspense resolving, so you should primarily use this for reacting to changes
    /// and long running tasks.
    ///
    /// Whenever the component that owns this future is dropped, the future will be dropped as well.
    ///
    /// Spawning a future onto the root scope will cause it to be dropped when the root component is dropped - which
    /// will only occur when the VirtualDom itself has been dropped.
    pub fn new(task: impl Future<Output = ()> + 'static) -> Self {
        spawn(task)
    }

    /// Drop the task immediately.
    ///
    /// This does not abort the task, so you'll want to wrap it in an abort handle if that's important to you
    pub fn stop(self) {
        remove_future(self);
    }
}

impl Runtime {
    /// Start a new future on the same thread as the rest of the VirtualDom.
    ///
    /// This future will not contribute to suspense resolving, so you should primarily use this for reacting to changes
    /// and long running tasks.
    ///
    /// Whenever the component that owns this future is dropped, the future will be dropped as well.
    ///
    /// Spawning a future onto the root scope will cause it to be dropped when the root component is dropped - which
    /// will only occur when the VirtualDom itself has been dropped.
    pub fn spawn(&self, scope: ScopeId, task: impl Future<Output = ()> + 'static) -> Task {
        let mut tasks = self.tasks.borrow_mut();

        let entry = tasks.vacant_entry();
        let task_id = Task(entry.key());

        let task = LocalTask {
            parent: self.current_task(),
            task: RefCell::new(Box::pin(task)),
            scope,
            waker: futures_util::task::waker(Arc::new(LocalTaskHandle {
                id: task_id,
                tx: self.sender.clone(),
            })),
        };

        let mut cx = std::task::Context::from_waker(&task.waker);

        if !task.task.borrow_mut().as_mut().poll(&mut cx).is_ready() {
            self.sender
                .unbounded_send(SchedulerMsg::TaskNotified(task_id))
                .expect("Scheduler should exist");
        }

        entry.insert(task);

        task_id
    }

    pub(crate) fn handle_task_wakeup(&self, id: Task) {
        let mut tasks = self.tasks.borrow_mut();

        let task = match tasks.get(id.0) {
            Some(task) => task,
            // The task was removed from the scheduler, so we can just ignore it
            None => return,
        };

        use std::task::Context;

        let mut cx = Context::from_waker(&task.waker);

        // update the scope stack
        self.scope_stack.borrow_mut().push(task.scope);
        self.rendering.set(false);
        self.current_task.set(Some(id));

        // If the task completes...
        if task.task.borrow_mut().as_mut().poll(&mut cx).is_ready() {
            // Remove it from the scope so we dont try to double drop it when the scope dropes
            self.get_context(task.scope)
                .unwrap()
                .spawned_tasks
                .borrow_mut()
                .remove(&id);

            // Remove it from the scheduler
            tasks.try_remove(id.0);
        }

        // Remove the scope from the stack
        self.scope_stack.borrow_mut().pop();
        self.rendering.set(true);
        self.current_task.set(None);
    }

    /// Take a queued task from the scheduler
    pub(crate) fn take_queued_task(&self) -> Option<Task> {
        self.queued_tasks.borrow_mut().pop_front()
    }

    /// Drop the future with the given TaskId
    ///
    /// This does not abort the task, so you'll want to wrap it in an abort handle if that's important to you
    pub(crate) fn remove_task(&self, id: Task) -> Option<LocalTask> {
        let task = self.tasks.borrow_mut().try_remove(id.0);

        // Remove the task from the queued tasks so we don't poll a different task with the same id
        self.queued_tasks.borrow_mut().retain(|t| *t != id);

        task
    }

    /// Get the currently running task
    pub fn current_task(&self) -> Option<Task> {
        self.current_task.get()
    }

    /// Get the parent task of the given task, if it exists
    pub fn parent_task(&self, task: Task) -> Option<Task> {
        self.tasks.borrow().get(task.0)?.parent
    }
}

/// the task itself is the waker
pub(crate) struct LocalTask {
    scope: ScopeId,
    parent: Option<Task>,
    task: RefCell<Pin<Box<dyn Future<Output = ()> + 'static>>>,
    waker: Waker,
}

/// The type of message that can be sent to the scheduler.
///
/// These messages control how the scheduler will process updates to the UI.
#[derive(Debug)]
pub(crate) enum SchedulerMsg {
    /// Immediate updates from Components that mark them as dirty
    Immediate(ScopeId),

    /// A task has woken and needs to be progressed
    TaskNotified(Task),
}

struct LocalTaskHandle {
    id: Task,
    tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
}

impl ArcWake for LocalTaskHandle {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // This can fail if the scheduler has been dropped while the application is shutting down
        let _ = arc_self
            .tx
            .unbounded_send(SchedulerMsg::TaskNotified(arc_self.id));
    }
}

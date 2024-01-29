use crate::innerlude::{remove_future, spawn, Runtime};
use crate::ScopeId;
use futures_util::task::ArcWake;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Waker;
use std::{cell::Cell, future::Future};
use std::{cell::RefCell, rc::Rc};

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

    /// Pause the task.
    pub fn pause(&self) {
        Runtime::with(|rt| rt.tasks.borrow()[self.0].active.set(false));
    }

    /// Check if the task is paused.
    pub fn paused(&self) -> bool {
        Runtime::with(|rt| !rt.tasks.borrow()[self.0].active.get()).unwrap_or_default()
    }

    pub fn wake(&self) {
        Runtime::with(|rt| _ = rt.sender.unbounded_send(SchedulerMsg::TaskNotified(*self)));
    }

    pub fn set_active(&self, active: bool) {
        Runtime::with(|rt| rt.tasks.borrow()[self.0].active.set(active));
    }

    /// Resume the task.
    pub fn resume(&self) {
        Runtime::with(|rt| {
            // set the active flag, and then ping the scheduler to ensure the task gets queued
            let was_active = rt.tasks.borrow()[self.0].active.replace(true);
            if !was_active {
                _ = rt.sender.unbounded_send(SchedulerMsg::TaskNotified(*self));
            }
        });
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
        // Insert the task, temporarily holding a borrow on the tasks map
        let (task, task_id) = {
            let mut tasks = self.tasks.borrow_mut();

            let entry = tasks.vacant_entry();
            let task_id = Task(entry.key());

            let task = Rc::new(LocalTask {
                scope,
                active: Cell::new(true),
                parent: self.current_task(),
                task: RefCell::new(Box::pin(task)),
                waker: futures_util::task::waker(Arc::new(LocalTaskHandle {
                    id: task_id,
                    tx: self.sender.clone(),
                })),
            });

            entry.insert(task.clone());

            (task, task_id)
        };

        // Get a borrow on the task, holding no borrows on the tasks map
        debug_assert!(self.tasks.try_borrow_mut().is_ok());
        debug_assert!(task.task.try_borrow_mut().is_ok());

        self.sender
            .unbounded_send(SchedulerMsg::TaskNotified(task_id))
            .expect("Scheduler should exist");

        task_id
    }

    /// Get the currently running task
    pub fn current_task(&self) -> Option<Task> {
        self.current_task.get()
    }

    /// Get the parent task of the given task, if it exists
    pub fn parent_task(&self, task: Task) -> Option<Task> {
        self.tasks.borrow().get(task.0)?.parent
    }

    /// Add this task to the queue of tasks that will manually get poked when the scheduler is flushed
    pub(crate) fn add_to_flush_table(&self) -> Task {
        let value = self.current_task().unwrap();
        self.flush_table.borrow_mut().insert(value);
        value
    }

    pub(crate) fn handle_task_wakeup(&self, id: Task) {
        debug_assert!(Runtime::current().is_some(), "Must be in a dioxus runtime");

        let task = self.tasks.borrow().get(id.0).cloned();

        // The task was removed from the scheduler, so we can just ignore it
        let Some(task) = task else {
            return;
        };

        // If a task woke up but is paused, we can just ignore it
        if !task.active.get() {
            return;
        }

        let mut cx = std::task::Context::from_waker(&task.waker);

        // update the scope stack
        self.scope_stack.borrow_mut().push(task.scope);
        self.rendering.set(false);
        self.current_task.set(Some(id));

        if task.task.borrow_mut().as_mut().poll(&mut cx).is_ready() {
            // Remove it from the scope so we dont try to double drop it when the scope dropes
            self.get_state(task.scope)
                .unwrap()
                .spawned_tasks
                .borrow_mut()
                .remove(&id);

            // Remove it from the scheduler
            self.tasks.borrow_mut().try_remove(id.0);
        }

        // Remove the scope from the stack
        self.scope_stack.borrow_mut().pop();
        self.rendering.set(true);
        self.current_task.set(None);
    }

    /// Drop the future with the given TaskId
    ///
    /// This does not abort the task, so you'll want to wrap it in an abort handle if that's important to you
    pub(crate) fn remove_task(&self, id: Task) -> Option<Rc<LocalTask>> {
        self.tasks.borrow_mut().try_remove(id.0)
    }
}

/// the task itself is the waker
pub(crate) struct LocalTask {
    scope: ScopeId,
    parent: Option<Task>,
    task: RefCell<Pin<Box<dyn Future<Output = ()> + 'static>>>,
    waker: Waker,
    active: Cell<bool>,
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
        _ = arc_self
            .tx
            .unbounded_send(SchedulerMsg::TaskNotified(arc_self.id));
    }
}

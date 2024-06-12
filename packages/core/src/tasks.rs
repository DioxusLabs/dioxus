use crate::innerlude::Effect;
use crate::innerlude::ScopeOrder;
use crate::innerlude::{remove_future, spawn, Runtime};
use crate::ScopeId;
use futures_util::task::ArcWake;
use slotmap::DefaultKey;
use std::sync::Arc;
use std::task::Waker;
use std::{cell::Cell, future::Future};
use std::{cell::RefCell, rc::Rc};
use std::{pin::Pin, task::Poll};

/// A task's unique identifier.
///
/// `Task` is a unique identifier for a task that has been spawned onto the runtime. It can be used to cancel the task
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Task(pub(crate) slotmap::DefaultKey);

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
    pub fn cancel(self) {
        remove_future(self);
    }

    /// Pause the task.
    pub fn pause(&self) {
        self.set_active(false);
    }

    /// Resume the task.
    pub fn resume(&self) {
        self.set_active(true);
    }

    /// Check if the task is paused.
    pub fn paused(&self) -> bool {
        Runtime::with(|rt| {
            if let Some(task) = rt.tasks.borrow().get(self.0) {
                !task.active.get()
            } else {
                false
            }
        })
        .unwrap_or_default()
    }

    /// Wake the task.
    pub fn wake(&self) {
        Runtime::with(|rt| _ = rt.sender.unbounded_send(SchedulerMsg::TaskNotified(*self)));
    }

    /// Poll the task immediately.
    pub fn poll_now(&self) -> Poll<()> {
        Runtime::with(|rt| rt.handle_task_wakeup(*self)).unwrap()
    }

    /// Set the task as active or paused.
    pub fn set_active(&self, active: bool) {
        Runtime::with(|rt| {
            if let Some(task) = rt.tasks.borrow().get(self.0) {
                let was_active = task.active.replace(active);
                if !was_active && active {
                    _ = rt.sender.unbounded_send(SchedulerMsg::TaskNotified(*self));
                }
            }
        });
    }
}

impl Runtime {
    /// Start a new future on the same thread as the rest of the VirtualDom.
    ///
    /// **You should generally use `spawn` instead of this method unless you specifically need to need to run a task during suspense**
    ///
    /// This future will not contribute to suspense resolving but it will run during suspense.
    ///
    /// Because this future runs during suspense, you need to be careful to work with hydration. It is not recommended to do any async IO work in this future, as it can easily cause hydration issues. However, you can use isomorphic tasks to do work that can be consistently replicated on the server and client like logging or responding to state changes.
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// // ❌ Do not do requests in isomorphic tasks. It may resolve at a different time on the server and client, causing hydration issues.
    /// let mut state = use_signal(|| None);
    /// spawn_isomorphic(async move {
    ///     state.set(Some(reqwest::get("https://api.example.com").await));
    /// });
    ///
    /// // ✅ You may wait for a signal to change and then log it
    /// let mut state = use_signal(|| 0);
    /// spawn_isomorphic(async move {
    ///     loop {
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///         println!("State is {state}");
    ///     }
    /// });
    /// ```
    pub fn spawn_isomorphic(
        &self,
        scope: ScopeId,
        task: impl Future<Output = ()> + 'static,
    ) -> Task {
        self.spawn_task_of_type(scope, task, TaskType::Isomorphic)
    }

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
        self.spawn_task_of_type(scope, task, TaskType::ClientOnly)
    }

    fn spawn_task_of_type(
        &self,
        scope: ScopeId,
        task: impl Future<Output = ()> + 'static,
        ty: TaskType,
    ) -> Task {
        // Insert the task, temporarily holding a borrow on the tasks map
        let (task, task_id) = {
            let mut tasks = self.tasks.borrow_mut();

            let mut task_id = Task(DefaultKey::default());
            let mut local_task = None;
            tasks.insert_with_key(|key| {
                task_id = Task(key);

                let new_task = Rc::new(LocalTask {
                    scope,
                    active: Cell::new(true),
                    parent: self.current_task(),
                    task: RefCell::new(Box::pin(task)),
                    waker: futures_util::task::waker(Arc::new(LocalTaskHandle {
                        id: task_id,
                        tx: self.sender.clone(),
                    })),
                    ty: Cell::new(ty),
                });

                local_task = Some(new_task.clone());

                new_task
            });

            (local_task.unwrap(), task_id)
        };

        // Get a borrow on the task, holding no borrows on the tasks map
        debug_assert!(self.tasks.try_borrow_mut().is_ok());
        debug_assert!(task.task.try_borrow_mut().is_ok());

        self.sender
            .unbounded_send(SchedulerMsg::TaskNotified(task_id))
            .expect("Scheduler should exist");

        task_id
    }

    /// Queue an effect to run after the next render
    pub(crate) fn queue_effect(&self, id: ScopeId, f: impl FnOnce() + 'static) {
        // Add the effect to the queue of effects to run after the next render for the given scope
        let mut effects = self.pending_effects.borrow_mut();
        let scope_order = ScopeOrder::new(id.height(), id);
        match effects.get(&scope_order) {
            Some(effects) => effects.push_back(Box::new(f)),
            None => {
                effects.insert(Effect::new(scope_order, f));
            }
        }
    }

    /// Get the currently running task
    pub fn current_task(&self) -> Option<Task> {
        self.current_task.get()
    }

    /// Get the parent task of the given task, if it exists
    pub fn parent_task(&self, task: Task) -> Option<Task> {
        self.tasks.borrow().get(task.0)?.parent
    }

    pub(crate) fn task_scope(&self, task: Task) -> Option<ScopeId> {
        self.tasks.borrow().get(task.0).map(|t| t.scope)
    }

    pub(crate) fn handle_task_wakeup(&self, id: Task) -> Poll<()> {
        debug_assert!(Runtime::current().is_some(), "Must be in a dioxus runtime");

        let task = self.tasks.borrow().get(id.0).cloned();

        // The task was removed from the scheduler, so we can just ignore it
        let Some(task) = task else {
            return Poll::Ready(());
        };

        // If a task woke up but is paused, we can just ignore it
        if !task.active.get() {
            return Poll::Pending;
        }

        let mut cx = std::task::Context::from_waker(&task.waker);

        // update the scope stack
        self.scope_stack.borrow_mut().push(task.scope);
        self.rendering.set(false);
        self.current_task.set(Some(id));

        let poll_result = task.task.borrow_mut().as_mut().poll(&mut cx);

        if poll_result.is_ready() {
            // Remove it from the scope so we dont try to double drop it when the scope dropes
            self.get_state(task.scope)
                .unwrap()
                .spawned_tasks
                .borrow_mut()
                .remove(&id);

            self.remove_task(id);
        }

        // Remove the scope from the stack
        self.scope_stack.borrow_mut().pop();
        self.rendering.set(true);
        self.current_task.set(None);

        poll_result
    }

    /// Drop the future with the given Task
    ///
    /// This does not abort the task, so you'll want to wrap it in an abort handle if that's important to you
    pub(crate) fn remove_task(&self, id: Task) -> Option<Rc<LocalTask>> {
        let task = self.tasks.borrow_mut().remove(id.0);
        if let Some(task) = &task {
            if task.suspended() {
                self.suspended_tasks.set(self.suspended_tasks.get() - 1);
            }
        }
        task
    }

    /// Check if a task should be run during suspense
    pub(crate) fn task_runs_during_suspense(&self, task: Task) -> bool {
        let borrow = self.tasks.borrow();
        let task: Option<&LocalTask> = borrow.get(task.0).map(|t| &**t);
        matches!(task, Some(LocalTask { ty, .. }) if ty.get().runs_during_suspense())
    }
}

/// the task itself is the waker
pub(crate) struct LocalTask {
    scope: ScopeId,
    parent: Option<Task>,
    task: RefCell<Pin<Box<dyn Future<Output = ()> + 'static>>>,
    waker: Waker,
    ty: Cell<TaskType>,
    active: Cell<bool>,
}

impl LocalTask {
    pub(crate) fn suspend(&self) {
        self.ty.set(TaskType::Suspended);
    }

    pub(crate) fn suspended(&self) -> bool {
        matches!(self.ty.get(), TaskType::Suspended)
    }
}

#[derive(Clone, Copy)]
enum TaskType {
    ClientOnly,
    Suspended,
    Isomorphic,
}

impl TaskType {
    fn runs_during_suspense(self) -> bool {
        matches!(self, TaskType::Isomorphic | TaskType::Suspended)
    }
}

/// The type of message that can be sent to the scheduler.
///
/// These messages control how the scheduler will process updates to the UI.
pub(crate) enum SchedulerMsg {
    /// Immediate updates from Components that mark them as dirty
    Immediate(ScopeId),

    /// A task has woken and needs to be progressed
    TaskNotified(Task),

    /// An effect has been queued to run after the next render
    EffectQueued,
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

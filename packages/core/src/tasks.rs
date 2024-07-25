use crate::innerlude::Effect;
use crate::innerlude::ScopeOrder;
use crate::innerlude::{remove_future, spawn, Runtime};
use crate::scope_context::ScopeStatus;
use crate::scope_context::SuspenseLocation;
use crate::ScopeId;
use futures_util::task::ArcWake;
use slotmap::DefaultKey;
use std::marker::PhantomData;
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
pub struct Task {
    pub(crate) id: slotmap::DefaultKey,
    // We add a raw pointer to make this !Send + !Sync
    unsend: PhantomData<*const ()>,
}

impl Task {
    /// Create a task from a raw id
    pub(crate) const fn from_id(id: slotmap::DefaultKey) -> Self {
        Self {
            id,
            unsend: PhantomData,
        }
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
            if let Some(task) = rt.tasks.borrow().get(self.id) {
                !task.active.get()
            } else {
                false
            }
        })
        .unwrap_or_default()
    }

    /// Wake the task.
    pub fn wake(&self) {
        Runtime::with(|rt| {
            _ = rt
                .sender
                .unbounded_send(SchedulerMsg::TaskNotified(self.id))
        })
        .unwrap();
    }

    /// Poll the task immediately.
    pub fn poll_now(&self) -> Poll<()> {
        Runtime::with(|rt| rt.handle_task_wakeup(*self)).unwrap()
    }

    /// Set the task as active or paused.
    pub fn set_active(&self, active: bool) {
        Runtime::with(|rt| {
            if let Some(task) = rt.tasks.borrow().get(self.id) {
                let was_active = task.active.replace(active);
                if !was_active && active {
                    _ = rt
                        .sender
                        .unbounded_send(SchedulerMsg::TaskNotified(self.id));
                }
            }
        })
        .unwrap();
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

            let mut task_id = Task::from_id(DefaultKey::default());
            let mut local_task = None;
            tasks.insert_with_key(|key| {
                task_id = Task::from_id(key);

                let new_task = Rc::new(LocalTask {
                    scope,
                    active: Cell::new(true),
                    parent: self.current_task(),
                    task: RefCell::new(Box::pin(task)),
                    waker: futures_util::task::waker(Arc::new(LocalTaskHandle {
                        id: task_id.id,
                        tx: self.sender.clone(),
                    })),
                    ty: RefCell::new(ty),
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
            .unbounded_send(SchedulerMsg::TaskNotified(task_id.id))
            .expect("Scheduler should exist");

        task_id
    }

    /// Queue an effect to run after the next render
    pub(crate) fn queue_effect(&self, id: ScopeId, f: impl FnOnce() + 'static) {
        let effect = Box::new(f) as Box<dyn FnOnce() + 'static>;
        let Some(scope) = self.get_state(id) else {
            return;
        };
        let mut status = scope.status.borrow_mut();
        match &mut *status {
            ScopeStatus::Mounted => {
                self.queue_effect_on_mounted_scope(id, effect);
            }
            ScopeStatus::Unmounted { effects_queued, .. } => {
                effects_queued.push(effect);
            }
        }
    }

    /// Queue an effect to run after the next render without checking if the scope is mounted
    pub(crate) fn queue_effect_on_mounted_scope(
        &self,
        id: ScopeId,
        f: Box<dyn FnOnce() + 'static>,
    ) {
        // Add the effect to the queue of effects to run after the next render for the given scope
        let mut effects = self.pending_effects.borrow_mut();
        let scope_order = ScopeOrder::new(id.height(), id);
        match effects.get(&scope_order) {
            Some(effects) => effects.push_back(f),
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
        self.tasks.borrow().get(task.id)?.parent
    }

    pub(crate) fn task_scope(&self, task: Task) -> Option<ScopeId> {
        self.tasks.borrow().get(task.id).map(|t| t.scope)
    }

    pub(crate) fn handle_task_wakeup(&self, id: Task) -> Poll<()> {
        #[cfg(feature = "debug_assertions")]
        {
            // Ensure we are currently inside a `Runtime`.
            Runtime::current().unwrap();
        }

        let task = self.tasks.borrow().get(id.id).cloned();

        // The task was removed from the scheduler, so we can just ignore it
        let Some(task) = task else {
            return Poll::Ready(());
        };

        // If a task woke up but is paused, we can just ignore it
        if !task.active.get() {
            return Poll::Pending;
        }

        let mut cx = std::task::Context::from_waker(&task.waker);

        // poll the future with the scope on the stack
        let poll_result = self.with_scope_on_stack(task.scope, || {
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

            poll_result
        });
        self.rendering.set(true);
        self.current_task.set(None);

        poll_result
    }

    /// Drop the future with the given Task
    ///
    /// This does not abort the task, so you'll want to wrap it in an abort handle if that's important to you
    pub(crate) fn remove_task(&self, id: Task) -> Option<Rc<LocalTask>> {
        // Remove the task from the task list
        let task = self.tasks.borrow_mut().remove(id.id);

        if let Some(task) = &task {
            // Remove the task from suspense
            if let TaskType::Suspended { boundary } = &*task.ty.borrow() {
                self.suspended_tasks.set(self.suspended_tasks.get() - 1);
                if let SuspenseLocation::UnderSuspense(boundary) = boundary {
                    boundary.remove_suspended_task(id);
                }
            }

            // Remove the task from pending work. We could reuse the slot before the task is polled and discarded so we need to remove it from pending work instead of filtering out dead tasks when we try to poll them
            if let Some(scope) = self.get_state(task.scope) {
                let order = ScopeOrder::new(scope.height(), scope.id);
                if let Some(dirty_tasks) = self.dirty_tasks.borrow_mut().get(&order) {
                    dirty_tasks.remove(id);
                }
            }
        }

        task
    }

    /// Check if a task should be run during suspense
    pub(crate) fn task_runs_during_suspense(&self, task: Task) -> bool {
        let borrow = self.tasks.borrow();
        let task: Option<&LocalTask> = borrow.get(task.id).map(|t| &**t);
        matches!(task, Some(LocalTask { ty, .. }) if ty.borrow().runs_during_suspense())
    }
}

/// the task itself is the waker
pub(crate) struct LocalTask {
    scope: ScopeId,
    parent: Option<Task>,
    task: RefCell<Pin<Box<dyn Future<Output = ()> + 'static>>>,
    waker: Waker,
    ty: RefCell<TaskType>,
    active: Cell<bool>,
}

impl LocalTask {
    /// Suspend the task, returns true if the task was already suspended
    pub(crate) fn suspend(&self, boundary: SuspenseLocation) -> bool {
        // Make this a suspended task so it runs during suspense
        let old_type = self.ty.replace(TaskType::Suspended { boundary });
        matches!(old_type, TaskType::Suspended { .. })
    }
}

#[derive(Clone)]
enum TaskType {
    ClientOnly,
    Suspended { boundary: SuspenseLocation },
    Isomorphic,
}

impl TaskType {
    fn runs_during_suspense(&self) -> bool {
        matches!(self, TaskType::Isomorphic | TaskType::Suspended { .. })
    }
}

/// The type of message that can be sent to the scheduler.
///
/// These messages control how the scheduler will process updates to the UI.
#[derive(Debug)]
pub(crate) enum SchedulerMsg {
    /// Immediate updates from Components that mark them as dirty
    Immediate(ScopeId),

    /// A task has woken and needs to be progressed
    TaskNotified(slotmap::DefaultKey),

    /// An effect has been queued to run after the next render
    EffectQueued,
}

struct LocalTaskHandle {
    id: slotmap::DefaultKey,
    tx: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,
}

impl ArcWake for LocalTaskHandle {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        _ = arc_self
            .tx
            .unbounded_send(SchedulerMsg::TaskNotified(arc_self.id));
    }
}

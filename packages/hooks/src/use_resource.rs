#![allow(missing_docs)]

use crate::{use_callback, use_signal, UseCallback};
use dioxus_core::{
    prelude::{spawn, use_hook},
    Task,
};
use dioxus_signals::*;
use futures_util::{future, pin_mut, FutureExt};
use std::future::Future;

/// A memo that resolve to a value asynchronously.
///
/// This runs on the server
#[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
pub fn use_resource<T, F>(future: impl Fn() -> F + 'static) -> Resource<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let mut value = use_signal(|| None);
    let mut state = use_signal(|| UseResourceState::Pending);
    let rc = use_hook(ReactiveContext::new);

    let mut cb = use_callback(move || {
        // Create the user's task
        #[allow(clippy::redundant_closure)]
        let fut = rc.run_in(|| future());

        // Spawn a wrapper task that polls the innner future and watch its dependencies
        spawn(async move {
            // move the future here and pin it so we can poll it
            let fut = fut;
            pin_mut!(fut);

            // Run each poll in the context of the reactive scope
            // This ensures the scope is properly subscribed to the future's dependencies
            let res = future::poll_fn(|cx| rc.run_in(|| fut.poll_unpin(cx))).await;

            // Set the value and state
            state.set(UseResourceState::Ready);
            value.set(Some(res));
        })
    });

    let mut task = use_hook(|| Signal::new(cb.call()));

    use_hook(|| {
        spawn(async move {
            loop {
                // Wait for the dependencies to change
                rc.changed().await;

                // Stop the old task
                task.write().cancel();

                // Start a new task
                task.set(cb.call());
            }
        })
    });

    Resource {
        task,
        value,
        state,
        callback: cb,
    }
}

#[allow(unused)]
pub struct Resource<T: 'static> {
    value: Signal<Option<T>>,
    task: Signal<Task>,
    state: Signal<UseResourceState>,
    callback: UseCallback<Task>,
}

/// A signal that represents the state of a future
// we might add more states (panicked, etc)
#[derive(Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub enum UseResourceState {
    /// The future is still running
    Pending,

    /// The future has been forcefully stopped
    Stopped,

    /// The future has been paused, tempoarily
    Paused,

    /// The future has completed
    Ready,
}

impl<T> Resource<T> {
    /// Restart the future with new dependencies.
    ///
    /// Will not cancel the previous future, but will ignore any values that it
    /// generates.
    pub fn restart(&mut self) {
        self.task.write().cancel();
        let new_task = self.callback.call();
        self.task.set(new_task);
    }

    /// Forcefully cancel a future
    pub fn cancel(&mut self) {
        self.state.set(UseResourceState::Stopped);
        self.task.write().cancel();
    }

    /// Pause the future
    pub fn pause(&mut self) {
        self.state.set(UseResourceState::Paused);
        self.task.write().pause();
    }

    /// Resume the future
    pub fn resume(&mut self) {
        if self.finished() {
            return;
        }

        self.state.set(UseResourceState::Pending);
        self.task.write().resume();
    }

    /// Get a handle to the inner task backing this future
    /// Modify the task through this handle will cause inconsistent state
    pub fn task(&self) -> Task {
        self.task.cloned()
    }

    /// Is the future currently finished running?
    ///
    /// Reading this does not subscribe to the future's state
    pub fn finished(&self) -> bool {
        matches!(
            *self.state.peek(),
            UseResourceState::Ready | UseResourceState::Stopped
        )
    }

    /// Get the current state of the future.
    pub fn state(&self) -> ReadOnlySignal<UseResourceState> {
        self.state.into()
    }

    /// Get the current value of the future.
    pub fn value(&self) -> ReadOnlySignal<Option<T>> {
        self.value.into()
    }
}

impl<T> std::ops::Deref for Resource<T> {
    type Target = Signal<Option<T>>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#![allow(missing_docs)]
use dioxus_core::{
    prelude::{spawn, use_hook},
    ScopeState, Task,
};
use dioxus_signals::*;
use futures_util::{future, pin_mut, FutureExt};
use std::{any::Any, cell::Cell, future::Future, pin::Pin, rc::Rc, sync::Arc, task::Poll};

/// A future that resolves to a value.
///
/// This runs the future only once - though the future may be regenerated
/// through the [`UseFuture::restart`] method.
///
/// This is commonly used for components that cannot be rendered until some
/// asynchronous operation has completed.
///
///
pub fn use_future<T, F>(mut future: impl FnMut() -> F) -> UseFuture<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let mut value = use_signal(|| None);
    let mut state = use_signal(|| UseFutureState::Pending);

    let task = use_signal(|| {
        // Create the user's task
        let fut = future();

        // Spawn a wrapper task that polls the innner future and watch its dependencies
        let task = spawn(async move {
            // move the future here and pin it so we can poll it
            let fut = fut;
            pin_mut!(fut);

            let res = future::poll_fn(|cx| {
                // Set the effect stack properly

                // Poll the inner future
                let ready = fut.poll_unpin(cx);

                // add any dependencies to the effect stack that we need to watch when restarting the future

                ready
            })
            .await;

            // Set the value
            value.set(Some(res));
        });

        Some(task)
    });

    UseFuture { task, value, state }
}

pub struct UseFuture<T: 'static> {
    value: Signal<Option<T>>,
    task: Signal<Option<Task>>,
    state: Signal<UseFutureState<T>>,
}

impl<T> UseFuture<T> {
    /// Restart the future with new dependencies.
    ///
    /// Will not cancel the previous future, but will ignore any values that it
    /// generates.
    pub fn restart(&self) {
        // self.needs_regen.set(true);
        // (self.update)();
    }

    /// Forcefully cancel a future
    pub fn cancel(&self) {
        // if let Some(task) = self.task.take() {
        //     cx.remove_future(task);
        // }
    }

    // Manually set the value in the future slot without starting the future over
    pub fn set(&mut self, new_value: T) {
        self.value.set(Some(new_value));
    }

    /// Return any value, even old values if the future has not yet resolved.
    ///
    /// If the future has never completed, the returned value will be `None`.
    pub fn value(&self) -> Signal<Option<T>> {
        self.value
    }

    /// Get the ID of the future in Dioxus' internal scheduler
    pub fn task(&self) -> Option<Task> {
        todo!()
        // self.task.get()
    }

    /// Get the current state of the future.
    pub fn state(&self) -> UseFutureState<T> {
        todo!()
        // match (&self.task.get(), &self.value()) {
        //     // If we have a task and an existing value, we're reloading
        //     (Some(_), Some(val)) => UseFutureState::Reloading(val),

        //     // no task, but value - we're done
        //     (None, Some(val)) => UseFutureState::Complete(val),

        //     // no task, no value - something's wrong? return pending
        //     (None, None) => UseFutureState::Pending,

        //     // Task, no value - we're still pending
        //     (Some(_), None) => UseFutureState::Pending,
        // }
    }
}

pub enum UseFutureState<T: 'static> {
    Pending,
    Complete(Signal<T>),
    Regenerating(Signal<T>), // the old value
}

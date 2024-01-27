#![allow(missing_docs)]
use dioxus_core::{
    prelude::{spawn, use_drop, use_hook},
    ScopeState, Task,
};
use dioxus_signals::*;
use futures_util::{future, pin_mut, FutureExt};
use std::{any::Any, cell::Cell, future::Future, pin::Pin, rc::Rc, sync::Arc, task::Poll};

/// A hook that allows you to spawn a future
///
/// Does not regenerate the future when dependencies change. If you're looking for a future that does, check out
/// `use_resource` instead.
pub fn use_future<F>(mut future: impl FnMut() -> F) -> UseFuture
where
    F: Future + 'static,
{
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
            // value.set(Some(res));
        });

        Some(task)
    });

    use_drop(move || {
        if let Some(task) = task.take() {
            task.stop();
        }
    });

    UseFuture { task, state }
}

pub struct UseFuture {
    task: Signal<Option<Task>>,
    state: Signal<UseFutureState>,
}

impl UseFuture {
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

    /// Get the ID of the future in Dioxus' internal scheduler
    pub fn task(&self) -> Option<Task> {
        todo!()
        // self.task.get()
    }

    /// Get the current state of the future.
    pub fn state(&self) -> UseFutureState {
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

pub enum UseFutureState {
    Pending,
    Complete,
    Regenerating, // the old value
}

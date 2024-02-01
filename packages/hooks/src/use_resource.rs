#![allow(missing_docs)]

use crate::use_signal;
use dioxus_core::{
    prelude::{spawn, suspend},
    Task,
};
use dioxus_signals::*;
use futures_util::{future, pin_mut, FutureExt};
use std::future::Future;

/// A memo that resolve to a value asynchronously.
///
/// Regular memos are synchronous and resolve immediately. However, you might want to resolve a memo
#[must_use = "Consider using `cx.spawn` to run a future without reading its value"]
pub fn use_async_memo<T, F>(future: impl Fn() -> F + 'static) -> AsyncMemo<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let mut value = use_signal(|| None);
    let state = use_signal(|| UseResourceState::Pending);

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
                // add any dependencies to the effect stack that we need to watch when restarting the future
                // Poll the inner future
                fut.poll_unpin(cx)
            })
            .await;

            // Set the value
            value.set(Some(Signal::new(res)));
        });

        Some(task)
    });

    AsyncMemo { task, value, state }
}

#[allow(unused)]
pub struct AsyncMemo<T: 'static> {
    value: Signal<Option<Signal<T>>>,
    task: Signal<Option<Task>>,
    state: Signal<UseResourceState<T>>,
}

impl<T> AsyncMemo<T> {
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
        todo!()
        // self.value.set(Some(new_value));
    }

    /// Return any value, even old values if the future has not yet resolved.
    ///
    /// If the future has never completed, the returned value will be `None`.
    pub fn value(&self) -> Option<Signal<T>> {
        self.value.cloned()
    }

    /// Get the ID of the future in Dioxus' internal scheduler
    pub fn task(&self) -> Option<Task> {
        todo!()
        // self.task.get()
    }

    /// Get the current state of the future.
    pub fn state(&self) -> UseResourceState<T> {
        todo!()
        // match (&self.task.get(), &self.value()) {
        //     // If we have a task and an existing value, we're reloading
        //     (Some(_), Some(val)) => UseResourceState::Reloading(val),

        //     // no task, but value - we're done
        //     (None, Some(val)) => UseResourceState::Complete(val),

        //     // no task, no value - something's wrong? return pending
        //     (None, None) => UseResourceState::Pending,

        //     // Task, no value - we're still pending
        //     (Some(_), None) => UseResourceState::Pending,
        // }
    }

    /// Wait for this async memo to resolve, returning the inner signal value
    /// If the value is pending, returns none and suspends the current component
    pub fn suspend(&self) -> Option<ReadOnlySignal<T>> {
        let out = self.value();
        if out.is_none() {
            suspend();
        }
        out.map(|sig| sig.into())
    }
}

pub enum UseResourceState<T: 'static> {
    Pending,
    Complete(Signal<T>),
    Regenerating(Signal<T>), // the old value
}

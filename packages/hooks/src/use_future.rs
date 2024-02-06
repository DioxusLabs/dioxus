#![allow(missing_docs)]
use crate::{use_callback, use_hook_did_run, use_signal, UseCallback};
use dioxus_core::{
    prelude::{flush_sync, spawn, use_hook},
    Task,
};
use dioxus_signals::*;
use dioxus_signals::{Readable, Writable};
use std::future::Future;

/// A hook that allows you to spawn a future
///
/// The future is spawned on the next call to `flush_sync` which means that it will not run on the server.
/// To run a future on the server, you should use `spawn` directly.
pub fn use_future<F>(mut future: impl FnMut() -> F + 'static) -> UseFuture
where
    F: Future + 'static,
{
    let mut state = use_signal(|| UseFutureState::Pending);

    let mut callback = use_callback(move || {
        let fut = future();
        spawn(async move {
            flush_sync().await;
            state.set(UseFutureState::Pending);
            fut.await;
            state.set(UseFutureState::Ready);
        })
    });

    // Create the task inside a copyvalue so we can reset it in-place later
    let task = use_hook(|| CopyValue::new(callback.call()));

    // Early returns in dioxus have consequences for use_memo, use_resource, and use_future, etc
    // We *don't* want futures to be running if the component early returns. It's a rather weird behavior to have
    // use_memo running in the background even if the component isn't hitting those hooks anymore.
    //
    // React solves this by simply not having early returns interleave with hooks.
    // However, since dioxus allows early returns (since we use them for suspense), we need to solve this problem
    use_hook_did_run(move |did_run| match did_run {
        true => task.peek().resume(),
        false => task.peek().pause(),
    });

    UseFuture {
        task,
        state,
        callback,
    }
}

#[derive(Clone, Copy)]
pub struct UseFuture {
    task: CopyValue<Task>,
    state: Signal<UseFutureState>,
    callback: UseCallback<Task>,
}

/// A signal that represents the state of a future
// we might add more states (panicked, etc)
#[derive(Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub enum UseFutureState {
    /// The future is still running
    Pending,

    /// The future has been forcefully stopped
    Stopped,

    /// The future has been paused, tempoarily
    Paused,

    /// The future has completed
    Ready,
}

impl UseFuture {
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
        self.state.set(UseFutureState::Stopped);
        self.task.write().cancel();
    }

    /// Pause the future
    pub fn pause(&mut self) {
        self.state.set(UseFutureState::Paused);
        self.task.write().pause();
    }

    /// Resume the future
    pub fn resume(&mut self) {
        if self.finished() {
            return;
        }

        self.state.set(UseFutureState::Pending);
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
            UseFutureState::Ready | UseFutureState::Stopped
        )
    }

    /// Get the current state of the future.
    pub fn state(&self) -> ReadOnlySignal<UseFutureState> {
        self.state.into()
    }
}

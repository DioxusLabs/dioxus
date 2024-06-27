#![allow(missing_docs)]
use crate::{use_callback, use_hook_did_run, use_signal, UseCallback};
use dioxus_core::prelude::*;
use dioxus_signals::*;
use std::future::Future;
use std::ops::Deref;

/// A hook that allows you to spawn a future the first time you render a component.
///
///
/// This future will **not** run on the server. To run a future on the server, you should use [`spawn_isomorphic`] directly.
///
///
/// `use_future` **won't return a value**. If you want to return a value from a future, use [`crate::use_resource()`] instead.
///
/// ## Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use std::time::Duration;
/// fn app() -> Element {
///     let mut count = use_signal(|| 0);
///     let mut running = use_signal(|| true);
///     // `use_future` will spawn an infinitely running future that can be started and stopped
///     use_future(move || async move {
///         loop {
///            if running() {
///                count += 1;
///            }
///            tokio::time::sleep(Duration::from_millis(400)).await;
///        }
///     });
///     rsx! {
///         div {
///             h1 { "Current count: {count}" }
///             button { onclick: move |_| running.toggle(), "Start/Stop the count"}
///             button { onclick: move |_| count.set(0), "Reset the count" }
///         }
///     }
/// }
/// ```
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
#[doc(alias = "use_async")]
pub fn use_future<F>(mut future: impl FnMut() -> F + 'static) -> UseFuture
where
    F: Future + 'static,
{
    let mut state = use_signal(|| UseFutureState::Pending);

    let callback = use_callback(move || {
        let fut = future();
        spawn(async move {
            state.set(UseFutureState::Pending);
            fut.await;
            state.set(UseFutureState::Ready);
        })
    });

    // Create the task inside a CopyValue so we can reset it in-place later
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

impl From<UseFuture> for ReadOnlySignal<UseFutureState> {
    fn from(val: UseFuture) -> Self {
        val.state.into()
    }
}

impl Readable for UseFuture {
    type Target = UseFutureState;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.state.try_read_unchecked()
    }

    #[track_caller]
    fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
        self.state.peek_unchecked()
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl Deref for UseFuture {
    type Target = dyn Fn() -> UseFutureState;

    fn deref(&self) -> &Self::Target {
        Readable::deref_impl(self)
    }
}

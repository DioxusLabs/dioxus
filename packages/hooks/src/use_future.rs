#![allow(missing_docs)]
use dioxus_core::{ScopeState, Task};
use dioxus_signals::{use_effect, use_signal, Signal};
use std::{any::Any, cell::Cell, future::Future, rc::Rc, sync::Arc};

/// A future that resolves to a value.
///
/// This runs the future only once - though the future may be regenerated
/// through the [`UseFuture::restart`] method.
///
/// This is commonly used for components that cannot be rendered until some
/// asynchronous operation has completed.
///
/// Whenever the hooks dependencies change, the future will be re-evaluated.
/// If a future is pending when the dependencies change, the previous future
/// will be canceled before the new one is started.
///
/// - dependencies: a tuple of references to values that are PartialEq + Clone
pub fn use_future<T, F>(future: impl FnMut() -> F) -> UseFuture<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let task = use_signal(|| None);

    use_effect(|| {
        // task.set();
    });
    //

    UseFuture {
        value: todo!(),
        task,
        state: todo!(),
    }
}

pub struct UseFuture<T: 'static> {
    value: Signal<T>,
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
    pub fn set(&self, new_value: T) {
        // self.state.set(Some(new_value));
    }

    /// Return any value, even old values if the future has not yet resolved.
    ///
    /// If the future has never completed, the returned value will be `None`.
    pub fn value(&self) -> Signal<Option<T>> {
        todo!()
        // self.state.current_val.as_ref().as_ref()
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

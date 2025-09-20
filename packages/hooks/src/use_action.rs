use crate::{use_callback, use_signal};
use dioxus_core::{use_hook, Callback, CapturedError, RenderError, Result, Task};
use dioxus_signals::{
    read_impls, CopyValue, ReadSignal, Readable, ReadableBoxExt, ReadableExt, ReadableRef, Signal,
    WritableExt,
};
use std::{cell::Ref, marker::PhantomData, prelude::rust_2024::Future};

pub fn use_action<F, E, I, O>(mut user_fn: impl FnMut(I) -> F + 'static) -> Action<I, O>
where
    F: Future<Output = Result<O, E>> + 'static,
    E: Into<CapturedError> + 'static,
    I: 'static,
    O: 'static,
{
    let mut value = use_signal(|| None as Option<O>);
    let mut error = use_signal(|| None as Option<CapturedError>);
    let mut task = use_signal(|| None as Option<Task>);
    let mut state = use_signal(|| ActionState::Unset);
    let callback = use_callback(move |input: I| {
        // Cancel any existing task
        if let Some(task) = task.take() {
            task.cancel();
        }

        // Spawn a new task, and *then* fire off the async
        let result = user_fn(input);
        let new_task = dioxus_core::spawn(async move {
            // Set the state to pending
            state.set(ActionState::Pending);

            // Create a new task
            let result = result.await;
            match result {
                Ok(res) => {
                    error.set(None);
                    value.set(Some(res));
                    state.set(ActionState::Ready);
                }
                Err(err) => {
                    error.set(Some(err.into()));
                    value.set(None);
                    state.set(ActionState::Errored);
                }
            }
        });

        task.set(Some(new_task));
    });

    // Create a reader that maps the Option<T> to T, unwrapping the Option
    // This should only be handed out if we know the value is Some. We never set the value back to None, only modify the state of the action
    let reader = use_hook(|| value.boxed().map(|v| v.as_ref().unwrap()).boxed());

    Action {
        value,
        error,
        task,
        callback,
        reader,
        _phantom: PhantomData,
        state,
    }
}

pub struct Action<I, T> {
    reader: ReadSignal<T>,
    error: Signal<Option<CapturedError>>,
    value: Signal<Option<T>>,
    task: Signal<Option<Task>>,
    callback: Callback<I, ()>,
    state: Signal<ActionState>,
    _phantom: PhantomData<*const I>,
}
impl<I: 'static, T: 'static> Action<I, T> {
    pub fn dispatch(&mut self, input: I) -> Dispatching<()> {
        (self.callback)(input);
        Dispatching(PhantomData)
    }

    pub fn value(&self) -> Option<ReadSignal<T>> {
        if *self.state.read() != ActionState::Ready {
            return None;
        }

        if self.value.read().is_none() {
            return None;
        }

        if self.error.read().is_some() {
            return None;
        }

        Some(self.reader)
    }

    pub fn result(&self) -> Option<Result<ReadSignal<T>, CapturedError>> {
        if !matches!(
            *self.state.read(),
            ActionState::Ready | ActionState::Errored
        ) {
            return None;
        }

        if let Some(err) = self.error.cloned() {
            return Some(Err(err));
        }

        if self.value.read().is_none() {
            return None;
        }

        Some(Ok(self.reader))
    }

    pub fn is_pending(&self) -> bool {
        *self.state.read() == ActionState::Pending
    }

    /// Clear the current value and error, setting the state to Reset
    pub fn reset(&mut self) {
        self.state.set(ActionState::Reset);
        if let Some(t) = self.task.take() {
            t.cancel()
        }
    }
}

impl<I, T> std::fmt::Debug for Action<I, T>
where
    T: std::fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.debug_struct("Action")
                .field("state", &self.state.read())
                .field("value", &self.value.read())
                .field("error", &self.error.read())
                .finish()
        } else {
            std::fmt::Debug::fmt(&self.value.read().as_ref(), f)
        }
    }
}
pub struct Dispatching<I>(PhantomData<*const I>);
impl<T> std::future::Future for Dispatching<T> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::task::Poll::Ready(())
    }
}

impl<I, T> Copy for Action<I, T> {}
impl<I, T> Clone for Action<I, T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// The state of an action
///
/// We can never reset the state to Unset, only to Reset, otherwise the value reader would panic.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
enum ActionState {
    Unset,
    Pending,
    Ready,
    Errored,
    Reset,
}

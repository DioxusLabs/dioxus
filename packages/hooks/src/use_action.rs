use crate::{use_callback, use_signal};
use dioxus_core::{use_hook, Callback, CapturedError, Result, Task};
use dioxus_signals::{ReadSignal, ReadableBoxExt, ReadableExt, Signal, WritableExt};
use futures_channel::oneshot::Receiver;
use futures_util::{future::Shared, FutureExt};
use std::{marker::PhantomData, pin::Pin, prelude::rust_2024::Future, task::Poll};

/// Create an action that runs async work on demand, triggered by user interaction.
///
/// Unlike [`use_resource`](crate::use_resource()) which runs automatically when its reactive
/// dependencies change, `use_action` only runs when you explicitly call it. This makes it
/// the right choice for mutations, form submissions, button clicks, and any async work that
/// should happen in response to a user event rather than on mount or state change.
///
/// The closure you pass must return a `Future` whose output is `Result<T, E>`. The action
/// tracks the lifecycle for you: pending, ready, or errored.
///
/// ## Basic usage
///
/// Pass a server function (or any async closure) directly:
///
/// ```rust, ignore
/// let mut save = use_action(save_to_database);
///
/// rsx! {
///     button { onclick: move |_| save.call(form_data.clone()), "Save" }
///
///     if save.pending() {
///         p { "Saving..." }
///     }
///
///     if let Some(result) = save.value() {
///         match result {
///             Ok(data) => rsx! { p { "Saved: {data}" } },
///             Err(err) => rsx! { p { "Error: {err}" } },
///         }
///     }
/// }
/// ```
///
/// # With inline async closures
///
/// ```rust, ignore
/// let mut fetch_dog = use_action(move |breed: String| async move {
///     reqwest::get(format!("https://dog.ceo/api/breed/{breed}/images/random"))
///         .await?
///         .json::<DogImage>()
///         .await
/// });
/// ```
///
/// ## Automatic cancellation
///
/// Calling an action while a previous call is still pending automatically cancels the
/// in-flight task. Only the most recent call's result is kept. You can also cancel or
/// reset manually:
///
/// ```rust, ignore
/// save.cancel(); // cancel in-flight work, reset state
/// save.reset();  // same — cancel and clear the value
/// ```
///
/// ## When to use `use_action` vs `use_resource`
///
/// | | `use_action` | `use_resource` |
/// |---|---|---|
/// | **Runs** | When you call it | Automatically on mount / dependency change |
/// | **Good for** | Mutations, form submits, button clicks | Loading data to display |
/// | **Cancellation** | Auto-cancels previous call | Restarts on dependency change |
pub fn use_action<E, C, M>(mut user_fn: C) -> Action<C::Input, C::Output>
where
    E: Into<CapturedError> + 'static,
    C: ActionCallback<M, E>,
    M: 'static,
    C::Input: 'static,
    C::Output: 'static,
    C: 'static,
{
    let mut value = use_signal(|| None as Option<C::Output>);
    let mut error = use_signal(|| None as Option<CapturedError>);
    let mut task = use_signal(|| None as Option<Task>);
    let mut state = use_signal(|| ActionState::Unset);
    let callback = use_callback(move |input: C::Input| {
        // Cancel any existing task
        if let Some(task) = task.take() {
            task.cancel();
        }

        let (tx, rx) = futures_channel::oneshot::channel();
        let rx = rx.shared();

        // Spawn a new task, and *then* fire off the async
        let result = user_fn.call(input);
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

            tx.send(()).ok();
        });

        task.set(Some(new_task));

        rx
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

/// A handle to an async action created by [`use_action`].
///
/// Call it with `.call(...)` to dispatch work, read results with `.value()`,
/// and check progress with `.pending()`. Implements `Copy` so it can be moved
/// into multiple event handlers freely.
pub struct Action<I, T: 'static> {
    reader: ReadSignal<T>,
    error: Signal<Option<CapturedError>>,
    value: Signal<Option<T>>,
    task: Signal<Option<Task>>,
    callback: Callback<I, Shared<Receiver<()>>>,
    state: Signal<ActionState>,
    _phantom: PhantomData<*const I>,
}

/// The internal state of an action
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

impl<I: 'static, O: 'static> Action<I, O> {
    /// The result of the most recent call, if it has completed.
    ///
    /// Returns `None` while the action is pending or has never been called.
    /// Returns `Some(Ok(signal))` on success or `Some(Err(e))` on failure.
    /// The returned `ReadSignal` is reactive — reading it in RSX will
    /// subscribe the component to updates.
    pub fn value(&self) -> Option<Result<ReadSignal<O>, CapturedError>> {
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

    /// Returns `true` while a call is in flight.
    pub fn pending(&self) -> bool {
        *self.state.read() == ActionState::Pending
    }

    /// Clear the result and cancel any in-flight work.
    pub fn reset(&mut self) {
        self.state.set(ActionState::Reset);
        if let Some(t) = self.task.take() {
            t.cancel()
        }
    }

    /// Cancel the in-flight task without clearing the previous result's state.
    pub fn cancel(&mut self) {
        if let Some(t) = self.task.take() {
            t.cancel()
        }
        self.state.set(ActionState::Reset);
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

impl<I, T> PartialEq for Action<I, T> {
    fn eq(&self, other: &Self) -> bool {
        self.callback == other.callback
    }
}

pub struct Dispatching<I> {
    _phantom: PhantomData<*const I>,
    receiver: Shared<Receiver<()>>,
}

impl<T> Dispatching<T> {
    pub(crate) fn new(receiver: Shared<Receiver<()>>) -> Self {
        Self {
            _phantom: PhantomData,
            receiver,
        }
    }
}

impl<T> std::future::Future for Dispatching<T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.receiver.poll_unpin(_cx) {
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<I, T> Copy for Action<I, T> {}
impl<I, T> Clone for Action<I, T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub trait ActionCallback<Marker, Err> {
    type Input;
    type Output;
    fn call(
        &mut self,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, Err>> + 'static;
}

macro_rules! impl_action_callback {
    // Base case: zero args
    () => {
        impl<Func, Out, Fut, Err> ActionCallback<(Out,), Err> for Func
        where
            Func: FnMut() -> Fut,
            Fut: Future<Output = Result<Out, Err>> + 'static,
        {
            type Input = ();
            type Output = Out;
            fn call(
                &mut self,
                _input: Self::Input,
            ) -> impl Future<Output = Result<Self::Output, Err>> + 'static {
                (self)()
            }
        }

        impl<Out> Action<(), Out> {
            /// Dispatch the action with no arguments.
            pub fn call(&mut self) -> Dispatching<()> {
                Dispatching::new((self.callback).call(()))
            }
        }
    };

    // N-arg case
    ($($arg:ident),+) => {
        impl<Func, Out, $($arg,)+ Fut, Err> ActionCallback<($($arg,)+ Out), Err> for Func
        where
            Func: FnMut($($arg),+) -> Fut,
            Fut: Future<Output = Result<Out, Err>> + 'static,
        {
            type Input = ($($arg,)+);
            type Output = Out;
            fn call(
                &mut self,
                input: Self::Input,
            ) -> impl Future<Output = Result<Self::Output, Err>> + 'static {
                #[allow(non_snake_case)]
                let ($($arg,)+) = input;
                (self)($($arg),+)
            }
        }

        impl<$($arg: 'static,)+ Out> Action<($($arg,)+), Out> {
            /// Dispatch the action with the given arguments.
            #[allow(non_snake_case)]
            pub fn call(&mut self, $($arg: $arg),+) -> Dispatching<()> {
                Dispatching::new((self.callback).call(($($arg,)+)))
            }
        }
    };
}

impl_action_callback!();
impl_action_callback!(A);
impl_action_callback!(A, B);
impl_action_callback!(A, B, C);
impl_action_callback!(A, B, C, D);
impl_action_callback!(A, B, C, D, E);
impl_action_callback!(A, B, C, D, E, F);

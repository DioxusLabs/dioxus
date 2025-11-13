use crate::{use_callback, use_signal};
use dioxus_core::{use_hook, Callback, CapturedError, Result, Task};
use dioxus_signals::{ReadSignal, ReadableBoxExt, ReadableExt, Signal, WritableExt};
use futures_channel::oneshot::Receiver;
use futures_util::{future::Shared, FutureExt};
use std::{marker::PhantomData, pin::Pin, prelude::rust_2024::Future, task::Poll};

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

pub struct Action<I, T: 'static> {
    reader: ReadSignal<T>,
    error: Signal<Option<CapturedError>>,
    value: Signal<Option<T>>,
    task: Signal<Option<Task>>,
    callback: Callback<I, Shared<Receiver<()>>>,
    state: Signal<ActionState>,
    _phantom: PhantomData<*const I>,
}

impl<I: 'static, O: 'static> Action<I, O> {
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

    pub fn pending(&self) -> bool {
        *self.state.read() == ActionState::Pending
    }

    /// Clear the current value and error, setting the state to Reset
    pub fn reset(&mut self) {
        self.state.set(ActionState::Reset);
        if let Some(t) = self.task.take() {
            t.cancel()
        }
    }

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

pub trait ActionCallback<M, E> {
    type Input;
    type Output;
    fn call(
        &mut self,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, E>> + 'static;
}

impl<F, O, G, E> ActionCallback<(O,), E> for F
where
    F: FnMut() -> G,
    G: Future<Output = Result<O, E>> + 'static,
{
    type Input = ();
    type Output = O;
    fn call(
        &mut self,
        _input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, E>> + 'static {
        (self)()
    }
}

impl<F, O, A, G, E> ActionCallback<(A, O), E> for F
where
    F: FnMut(A) -> G,
    G: Future<Output = Result<O, E>> + 'static,
{
    type Input = (A,);
    type Output = O;
    fn call(
        &mut self,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, E>> + 'static {
        let (a,) = input;
        (self)(a)
    }
}

impl<O, A, B, F, G, E> ActionCallback<(A, B, O), E> for F
where
    F: FnMut(A, B) -> G,
    G: Future<Output = Result<O, E>> + 'static,
{
    type Input = (A, B);
    type Output = O;
    fn call(
        &mut self,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, E>> + 'static {
        let (a, b) = input;
        (self)(a, b)
    }
}

impl<O, A, B, C, F, G, E> ActionCallback<(A, B, C, O), E> for F
where
    F: FnMut(A, B, C) -> G,
    G: Future<Output = Result<O, E>> + 'static,
{
    type Input = (A, B, C);
    type Output = O;
    fn call(
        &mut self,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self::Output, E>> + 'static {
        let (a, b, c) = input;
        (self)(a, b, c)
    }
}

impl<O> Action<(), O> {
    pub fn call(&mut self) -> Dispatching<()> {
        Dispatching::new((self.callback).call(()))
    }
}

impl<A: 'static, O> Action<(A,), O> {
    pub fn call(&mut self, _a: A) -> Dispatching<()> {
        Dispatching::new((self.callback).call((_a,)))
    }
}

impl<A: 'static, B: 'static, O> Action<(A, B), O> {
    pub fn call(&mut self, _a: A, _b: B) -> Dispatching<()> {
        Dispatching::new((self.callback).call((_a, _b)))
    }
}

impl<A: 'static, B: 'static, C: 'static, O> Action<(A, B, C), O> {
    pub fn call(&mut self, _a: A, _b: B, _c: C) -> Dispatching<()> {
        Dispatching::new((self.callback).call((_a, _b, _c)))
    }
}

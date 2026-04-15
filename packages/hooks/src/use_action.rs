use crate::{use_callback, use_signal};
use dioxus_core::{Callback, CapturedError, Result, Task};
use dioxus_signals::{
    ProjectOption, ProjectResult, ReadSignal, ReadableExt, Signal, WritableExt,
};
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
    let mut result = use_signal(|| None as Option<Result<C::Output>>);
    let mut task = use_signal(|| None as Option<Task>);
    let callback = use_callback(move |input: C::Input| {
        // Cancel any existing task
        if let Some(task) = task.take() {
            task.cancel();
        }

        // Clear any previously completed value while this action is running.
        result.set(None);

        let (tx, rx) = futures_channel::oneshot::channel();
        let rx = rx.shared();

        // Spawn a new task, and *then* fire off the async
        let action_result = user_fn.call(input);
        let new_task = dioxus_core::spawn(async move {
            result.set(Some(action_result.await.map_err(Into::into)));
            task.set(None);

            tx.send(()).ok();
        });

        task.set(Some(new_task));

        rx
    });

    Action {
        result,
        task,
        callback,
        _phantom: PhantomData,
    }
}

pub struct Action<I, T: 'static> {
    result: Signal<Option<Result<T>>>,
    task: Signal<Option<Task>>,
    callback: Callback<I, Shared<Receiver<()>>>,
    _phantom: PhantomData<*const I>,
}

impl<I: 'static, O: 'static> Action<I, O> {
    /// Returns the current completed result as projected read signals.
    ///
    /// `None` means the action is unset, has been reset, or is currently pending.
    pub fn result(&self) -> Option<Result<ReadSignal<O>, ReadSignal<CapturedError>>> {
        self.result
            .transpose()
            .map(|resolved| resolved.transpose())
            .map(|result| result.map(Into::into).map_err(Into::into))
    }

    /// Returns the current completed value, cloning any captured error.
    pub fn value(&self) -> Option<Result<ReadSignal<O>, CapturedError>> {
        self.result()
            .map(|result| result.map_err(|error| error.cloned()))
    }

    /// Returns `true` while the action is running.
    pub fn pending(&self) -> bool {
        self.task.read().is_some()
    }

    /// Clear the current value and cancel any running task.
    pub fn reset(&mut self) {
        if let Some(t) = self.task.take() {
            t.cancel()
        }
        self.result.set(None);
    }

    /// Cancel the running task, if any, and clear the current value.
    pub fn cancel(&mut self) {
        self.reset();
    }
}

impl<I, T> std::fmt::Debug for Action<I, T>
where
    T: std::fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.debug_struct("Action")
                .field("pending", &self.task.read().is_some())
                .field("result", &self.result.read())
                .finish()
        } else {
            std::fmt::Debug::fmt(&self.result.read().as_ref(), f)
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

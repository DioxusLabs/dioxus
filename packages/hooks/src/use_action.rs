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
    let callback = use_callback(move |input: I| {
        // Cancel any existing task
        if let Some(task) = task.take() {
            task.cancel();
        }

        // Spawn a new task, and *then* fire off the async
        let result = user_fn(input);
        let new_task = dioxus_core::spawn(async move {
            // Create a new task
            let result = result.await;
            match result {
                Ok(res) => {
                    error.set(None);
                    value.set(Some(res));
                }
                Err(err) => {
                    error.set(Some(err.into()));
                    value.set(None);
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
    }
}

pub struct Action<I, T> {
    reader: ReadSignal<T>,
    error: Signal<Option<CapturedError>>,
    value: Signal<Option<T>>,
    task: Signal<Option<Task>>,
    callback: Callback<I, ()>,
    _phantom: PhantomData<*const I>,
}
impl<I: 'static, T: 'static> Action<I, T> {
    pub fn dispatch(&mut self, input: I) -> Dispatching<()> {
        (self.callback)(input);
        Dispatching(PhantomData)
    }

    pub fn value(&self) -> Option<ReadSignal<T>> {
        if self.value.peek().is_none() {
            return None;
        }

        if self.error.peek().is_some() {
            return None;
        }

        Some(self.reader)
    }

    pub fn result(&self) -> Option<Result<ReadSignal<T>, CapturedError>> {
        if let Some(err) = self.error.cloned() {
            return Some(Err(err));
        }

        if self.value.peek().is_none() {
            return None;
        }

        Some(Ok(self.reader))
    }

    pub fn is_pending(&self) -> bool {
        self.value().is_none() && self.task.peek().is_some()
    }
}

pub struct Dispatching<I>(PhantomData<*const I>);

impl<I, T> Copy for Action<I, T> {}
impl<I, T> Clone for Action<I, T> {
    fn clone(&self) -> Self {
        *self
    }
}

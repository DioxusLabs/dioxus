use crate::{use_callback, use_signal};
use dioxus_core::{Callback, CapturedError, RenderError, Result, Task};
use dioxus_signals::{
    read_impls, CopyValue, ReadSignal, Readable, ReadableExt, ReadableRef, Signal, WritableExt,
};
use std::{cell::Ref, marker::PhantomData, prelude::rust_2024::Future};

pub fn use_action<F, E, I, O>(mut user_fn: impl FnMut(I) -> F + 'static) -> Action<I, O>
where
    F: Future<Output = Result<O, E>> + 'static,
    O: 'static,
    E: Into<CapturedError> + 'static,
    I: 'static,
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

    Action {
        value,
        error,
        task,
        callback,
        _phantom: PhantomData,
    }
}

pub struct Action<I, T> {
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

    pub fn ok(&self) -> Option<Signal<T>> {
        todo!()
    }

    pub fn value(&self) -> Option<Signal<T>> {
        todo!()
    }

    pub fn result(&self) -> Result<Signal<Option<T>>, CapturedError> {
        if let Some(err) = self.error.cloned() {
            return Err(err);
        }

        Ok(self.value)
    }

    pub fn is_pending(&self) -> bool {
        todo!()
    }
}

pub struct Dispatching<I>(PhantomData<*const I>);

impl<I, T> Copy for Action<I, T> {}
impl<I, T> Clone for Action<I, T> {
    fn clone(&self) -> Self {
        *self
    }
}

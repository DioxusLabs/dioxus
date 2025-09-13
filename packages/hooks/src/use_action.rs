use dioxus_core::{CapturedError, RenderError, Result};
// use cr::Resource;
use dioxus_signals::{
    read_impls, CopyValue, ReadSignal, Readable, ReadableExt, ReadableRef, Signal, WritableExt,
};
use std::{marker::PhantomData, prelude::rust_2024::Future};

pub fn use_action<F: Future<Output = Result<O, E>>, E, I, O>(
    f: impl FnOnce(I) -> F,
) -> Action<I, O> {
    todo!()
}

pub struct Action<I, T> {
    _t: PhantomData<*const T>,
    _i: PhantomData<*const I>,
}
impl<I, T> Action<I, T> {
    pub fn dispatch(&mut self, input: I) -> Dispatching<()> {
        todo!()
    }

    pub fn ok(&self) -> Option<ReadSignal<T>> {
        todo!()
    }

    pub fn value(&self) -> Option<ReadSignal<T>> {
        todo!()
    }

    pub fn result(&self) -> Option<Result<ReadSignal<T>, CapturedError>> {
        todo!()
    }

    pub fn is_pending(&self) -> bool {
        todo!()
    }
}

pub struct Dispatching<I>(PhantomData<*const I>);
impl<T> std::future::Future for Dispatching<T> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}

impl<I, T> std::ops::Deref for Action<I, T> {
    type Target = fn(I);

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}

impl<I, T> Clone for Action<I, T> {
    fn clone(&self) -> Self {
        todo!()
    }
}
impl<I, T> Copy for Action<I, T> {}

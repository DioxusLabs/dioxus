use dioxus_core::{RenderError, Result};
// use cr::Resource;
use dioxus_signals::{Loader, Signal};
use std::{marker::PhantomData, prelude::rust_2024::Future};

/// A hook to create a resource that loads data asynchronously.
///
/// To bubble errors and pending, simply use `?` on the result of the resource read.
///
/// To inspect the state of the resource, you can use the RenderError enum along with the RenderResultExt trait.
pub fn use_loader<
    F: Future<Output = Result<T, E>>,
    T: 'static,
    // T: 'static + PartialEq,
    E: Into<dioxus_core::CapturedError>,
>(
    // pub fn use_loader<F: Future<Output = Result<T, E>>, T: 'static, E: Into<anyhow::Error>>(
    f: impl FnMut() -> F,
) -> Result<Loader<T>, Loading> {
    todo!()
}

#[derive(PartialEq)]
pub enum Loading {
    Pending(LoaderHandle<()>),

    Failed(LoaderHandle<RenderError>),
}

impl std::fmt::Debug for Loading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loading::Pending(_) => write!(f, "Loading::Pending"),
            Loading::Failed(_) => write!(f, "Loading::Failed"),
        }
    }
}

impl std::fmt::Display for Loading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loading::Pending(_) => write!(f, "Loading is still pending"),
            Loading::Failed(_) => write!(f, "Loading has failed"),
        }
    }
}

impl From<Loading> for RenderError {
    fn from(val: Loading) -> Self {
        todo!()
    }
}

#[derive(PartialEq)]
pub struct LoaderHandle<T> {
    _t: PhantomData<*const T>,
}
impl<T> LoaderHandle<T> {
    pub fn restart(&self) {
        todo!()
    }
}
impl<T> Clone for LoaderHandle<T> {
    fn clone(&self) -> Self {
        todo!()
    }
}
impl<T> Copy for LoaderHandle<T> {}

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

    pub fn value(&self) -> Option<Signal<T>> {
        todo!()
    }

    pub fn result(&self) -> Option<Result<Signal<T>>> {
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

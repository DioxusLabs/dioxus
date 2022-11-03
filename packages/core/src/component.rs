// pub trait IntoComponentType<T> {
//     fn into_component_type(self) -> ComponentType;
// }

use std::marker::PhantomData;

use futures_util::Future;

use crate::{scopes::Scope, Element};

pub type Component<T = ()> = fn(Scope<T>) -> Element;

pub enum ComponentFn<'a, T, F: Future<Output = Element<'a>> = Dummy<'a>> {
    Sync(fn(Scope<'a, T>) -> Element),
    Async(fn(Scope<'a, T>) -> F),
}

pub trait IntoComponent<'a, T, F: Future<Output = Element<'a>> = Dummy<'a>, A = ()> {
    fn into_component(self) -> ComponentFn<'a, T, F>;
}

impl<'a, T> IntoComponent<'a, T, Dummy<'a>> for fn(Scope<T>) -> Element {
    fn into_component(self) -> ComponentFn<'a, T> {
        ComponentFn::Sync(self)
    }
}

pub struct AsyncMarker;
impl<'a, T, F: Future<Output = Element<'a>>> IntoComponent<'a, T, F, AsyncMarker>
    for fn(Scope<'a, T>) -> F
{
    fn into_component(self) -> ComponentFn<'a, T, F> {
        ComponentFn::Async(self)
    }
}

pub struct Dummy<'a>(PhantomData<&'a ()>);
impl<'a> Future for Dummy<'a> {
    type Output = Element<'a>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        unreachable!()
    }
}

// pub trait IntoComponentType<T> {
//     fn into_component_type(self) -> ComponentType;
// }

use futures_util::Future;

use crate::{scopes::Scope, Element};

pub type Component<T = ()> = fn(Scope<T>) -> Element;

pub enum ComponentFn<T, F: Future<Output = ()> = Dummy> {
    Sync(fn(Scope<T>) -> Element),
    Async(fn(Scope<T>) -> F),
}

pub trait IntoComponent<T, A = ()> {
    fn into_component(self) -> ComponentFn<T>;
}

impl<T> IntoComponent<T> for fn(Scope<T>) -> Element {
    fn into_component(self) -> ComponentFn<T> {
        ComponentFn::Sync(self)
    }
}

pub struct AsyncMarker;
impl<'a, T, F: Future<Output = Element<'a>>> IntoComponent<T, AsyncMarker>
    for fn(&'a Scope<T>) -> F
{
    fn into_component(self) -> ComponentFn<T> {
        todo!()
    }
}

pub struct Dummy;
impl Future for Dummy {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        unreachable!()
    }
}

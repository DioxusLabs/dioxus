// pub trait IntoComponentType<T> {
//     fn into_component_type(self) -> ComponentType;
// }

use std::marker::PhantomData;

use futures_util::Future;

use crate::{scopes::Scope, Element};

pub type Component<'a, T = ()> = fn(Scope<'a, T>) -> Element<'a>;

pub enum ComponentFn<'a, T, F: Future<Output = Element<'a>> = Dummy<'a>> {
    Sync(fn(Scope<'a, T>) -> Element),
    Async(fn(Scope<'a, T>) -> F),
}

pub trait IntoComponent<'a, T, F: Future<Output = Element<'a>> = Dummy<'a>, A = ()> {
    fn into_component(self) -> ComponentFn<'a, T, F>;
}

impl<'a, T> IntoComponent<'a, T, Dummy<'a>> for fn(Scope<'a, T>) -> Element<'a> {
    fn into_component(self) -> ComponentFn<'a, T> {
        ComponentFn::Sync(self)
    }
}

enum ComponentFn2 {
    Sync(fn(Scope) -> Element),
}

trait AsComponent {
    fn as_component(self) -> ComponentFn2;
}

// impl AsComponent for fn(Scope) -> Element {
//     fn as_component(self) -> ComponentFn2 {
//         ComponentFn2::Sync(self)
//     }
// }

// impl<F> AsComponent for for<'r> fn(Scope<'r>) ->
// where
//     F: Future<Output = Element<'r>>,
// {
//     fn as_component(self) -> ComponentFn2 {
//         ComponentFn2::Sync(self)
//     }
// }

fn takes_f(f: impl AsComponent) {}

#[test]
fn example() {
    fn app(cx: Scope) -> Element {
        todo!()
    }

    // takes_f(app as fn(Scope) -> Element);
}

// pub struct AsyncMarker;
// impl<'a, T, F: Future<Output = Element<'a>>> IntoComponent<'a, T, F, AsyncMarker>
//     for fn(Scope<'a, T>) -> F
// {
//     fn into_component(self) -> ComponentFn<'a, T, F> {
//         ComponentFn::Async(self)
//     }
// }

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

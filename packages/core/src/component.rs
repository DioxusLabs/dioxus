// pub trait IntoComponentType<T> {
//     fn into_component_type(self) -> ComponentType;
// }

use std::marker::PhantomData;

use futures_util::Future;

use crate::{scopes::Scope, Element};

pub type Component<'a, T = ()> = fn(Scope<'a, T>) -> Element<'a>;

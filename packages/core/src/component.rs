// pub trait IntoComponentType<T> {
//     fn into_component_type(self) -> ComponentType;
// }

use crate::{element::Element, scopes::Scope};

pub type Component<T = ()> = fn(&Scope<T>) -> Element;

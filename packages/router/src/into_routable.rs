use url::Url;

use crate::prelude::Routable;

/// Something that can be converted into a [`NavigationTarget`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IntoRoutable(String);

impl IntoRoutable {
    fn FromStr(value: String) -> Self {
        Self(value)
    }

    fn Route<R: Routable>(value: R) -> Self {
        Self(value.serialize())
    }

    pub(crate) fn is_external(&self) -> bool {
        todo!()
    }
}

impl<R: Routable> From<R> for IntoRoutable {
    fn from(value: R) -> Self {
        IntoRoutable::Route(value)
    }
}

// impl<R: Routable> From<NavigationTarget<R>> for IntoRoutable {
//     fn from(value: NavigationTarget<R>) -> Self {
//         match value {
//             NavigationTarget::Internal(route) => IntoRoutable::Route(Rc::new(route) as Rc<dyn Any>),
//             NavigationTarget::External(url) => IntoRoutable::FromStr(url),
//         }
//     }
// }

// impl PartialEq for IntoRoutable {
//     fn eq(&self, other: &Self) -> bool {
//         match (self, other) {
//             (IntoRoutable::FromStr(a), IntoRoutable::FromStr(b)) => a == b,
//             (IntoRoutable::Route(a), IntoRoutable::Route(b)) => Rc::ptr_eq(a, b),
//             _ => false,
//         }
//     }
// }

impl From<String> for IntoRoutable {
    fn from(value: String) -> Self {
        IntoRoutable::FromStr(value)
    }
}

impl From<&String> for IntoRoutable {
    fn from(value: &String) -> Self {
        IntoRoutable::FromStr(value.to_string())
    }
}

impl From<&str> for IntoRoutable {
    fn from(value: &str) -> Self {
        IntoRoutable::FromStr(value.to_string())
    }
}

impl From<Url> for IntoRoutable {
    fn from(url: Url) -> Self {
        IntoRoutable::FromStr(url.to_string())
    }
}

impl From<&Url> for IntoRoutable {
    fn from(url: &Url) -> Self {
        IntoRoutable::FromStr(url.to_string())
    }
}

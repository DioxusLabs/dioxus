//

use dioxus::core::AttributeDescription;
use std::marker::PhantomData;

fn main() {
    let p: HtmlElement = HtmlElement::new("asd", None, false);
}

struct HtmlNamespace;

/// The raw definition of an element
///
/// This should be compiled away
pub struct CustomElement<N, E = ()> {
    pub tag: &'static str,
    pub namespace: Option<&'static str>,
    _t: PhantomData<(N, E)>,
}

type HtmlElement<T = ()> = CustomElement<HtmlNamespace, T>;

struct link;
impl HtmlElement<link> {
    pub const fn crossorigin(&self) -> AttributeDescription {
        AttributeDescription {
            name: "crossorigin",
            namespace: None,
        }
    }

    pub const fn href(&self) -> AttributeDescription {
        AttributeDescription {
            name: "href",
            namespace: None,
        }
    }

    pub const fn hreflang(&self) -> AttributeDescription {
        AttributeDescription {
            name: "hreflang",
            namespace: None,
        }
    }

    pub const fn integrity(&self) -> AttributeDescription {
        AttributeDescription {
            name: "integrity",
            namespace: None,
        }
    }
}

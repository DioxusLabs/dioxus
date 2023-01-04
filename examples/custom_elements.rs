//

use dioxus::core::{AttributeDescription, CustomElement};
use std::marker::PhantomData;

fn main() {
    let p: HtmlElement = HtmlElement::new("asd", None, false);
}

struct HtmlNamespace;

type HtmlElement<T = ()> = CustomElement<HtmlNamespace, T>;

struct link;
impl HtmlElement<link> {
    pub const fn crossorigin(&self) -> AttributeDescription {
        AttributeDescription {
            name: "crossorigin",
            namespace: None,
            is_boolean: false,
        }
    }

    pub const fn href(&self) -> AttributeDescription {
        AttributeDescription {
            name: "href",
            namespace: None,
            is_boolean: false,
        }
    }

    pub const fn hreflang(&self) -> AttributeDescription {
        AttributeDescription {
            name: "hreflang",
            namespace: None,
            is_boolean: false,
        }
    }

    pub const fn integrity(&self) -> AttributeDescription {
        AttributeDescription {
            name: "integrity",
            namespace: None,
            is_boolean: false,
        }
    }
}

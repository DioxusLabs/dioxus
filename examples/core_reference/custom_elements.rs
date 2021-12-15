//! Example: Web Components & Custom Elements
//! -----------------------------------------
//!
//! Web components are a flavor of html elements that can be user-defined.
//! See: https://www.webcomponents.org for more information.
//!
//! Users who use webcomponents typically don't use Dioxus. However, if you would like to use webcomponents in Dioxus,
//! you can easily create a new custom element with compile-time correct wrappers around the webcomponent.
//!
//! We don't support building new webcomponents with Dioxus, however.  :(

use dioxus::prelude::*;

pub static Example: Component<()> = |cx| {
    cx.render(rsx! {
        div {
            custom_element {
                custom_attr: "custom data on custom elements"
            }
        }
    })
};

mod dioxus_elements {
    use std::fmt::Arguments;

    use dioxus::prelude::DioxusElement;

    #[allow(non_camel_case_types)]
    pub struct custom_element;
    impl DioxusElement for custom_element {
        const TAG_NAME: &'static str = "custom_element";
        const NAME_SPACE: Option<&'static str> = None;
    }
    impl custom_element {
        pub fn custom_attr<'a>(&self, f: NodeFactory<'a>, val: Arguments) -> Attribute<'a> {
            f.attr("custom_asttr", val, None, false)
        }
    }

    // Re-export the normal html namespace
    pub use dioxus::prelude::dioxus_elements::*;
    use dioxus_core::{nodes::Attribute, NodeFactory};
}

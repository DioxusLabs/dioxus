//! Example: Web Components
//! -----------------------
//!
//! Web components are a flavor of html elements that can be user-defined.
//! See: https://www.webcomponents.org for more information.
//!
//! Users who use webcomponents typically don't use Dioxus. However, if you would like to use webcomponents in Dioxus,
//! you can easily create a new custom element with compile-time correct wrappers around the webcomponent.
//!
//! We don't support building new webcomponents with Dioxus, however.
//!

fn main() {}

mod dioxus_elements {
    use dioxus::prelude::DioxusElement;

    struct custom_element;
    impl DioxusElement for custom_element {
        const TAG_NAME: &'static str = "custom_element";
        const NAME_SPACE: Option<&'static str> = None;
    }

    // Re-export the normal html namespace
    pub use dioxus::prelude::dioxus_elements::*;
}

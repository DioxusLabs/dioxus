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

use dioxus::{builder::ElementBuilder, prelude::NodeFactory};

fn main() {}

// TODO
struct MyEle<'a, 'b> {
    el: ElementBuilder<'a, 'b>,
    fac: &'b NodeFactory<'a>,
}

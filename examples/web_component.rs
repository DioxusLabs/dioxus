//! Dioxus allows webcomponents to be created with a simple syntax.
//!
//! Read more about webcomponents [here](https://developer.mozilla.org/en-US/docs/Web/Web_Components)
//!
//! We typically suggest wrapping webcomponents in a strongly typed interface using a component.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "Web Components" }
            CoolWebComponet { my_prop: "Hello, world!".to_string() }
        }
    }
}

/// A web-component wrapped with a strongly typed interface using a component
#[component]
fn CoolWebComponet(my_prop: String) -> Element {
    rsx! {
        // rsx! takes a webcomponent as long as its tag name is separated with dashes
        web-component {
            // Since web-components don't have built-in attributes, the attribute names must be passed as a string
            "my-prop": my_prop,
        }
    }
}

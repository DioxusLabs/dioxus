//! This example demonstrates how to create a generic component in Dioxus.
//!
//! Generic components can be useful when you want to create a component that renders differently depending on the type
//! of data it receives. In this particular example, we're just using a type that implements `Display` and `PartialEq`,

use dioxus::prelude::*;
use std::fmt::Display;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    rsx! {
        generic_child { data: 0 }
    }
}

#[derive(PartialEq, Props, Clone)]
struct GenericChildProps<T: Display + PartialEq + Clone + 'static> {
    data: T,
}

fn generic_child<T: Display + PartialEq + Clone>(props: GenericChildProps<T>) -> Element {
    rsx! {
        div { "{props.data}" }
    }
}

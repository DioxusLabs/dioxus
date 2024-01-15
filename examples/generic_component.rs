use std::fmt::Display;

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    render! {
        generic_child { data: 0 }
    }
}

#[derive(PartialEq, Props, Clone)]
struct GenericChildProps<T: Display + PartialEq + Clone + 'static> {
    data: T,
}

fn generic_child<T: Display + PartialEq + Clone>(props: GenericChildProps<T>) -> Element {
    render! {
        div { "{props.data}" }
    }
}

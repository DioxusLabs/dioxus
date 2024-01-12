use std::fmt::Display;

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    render! {
        generic_child { data: 0 }
    }
}

#[derive(PartialEq, Props)]
struct GenericChildProps<T: Display + PartialEq> {
    data: T,
}

fn generic_child<T: Display + PartialEq>(cx: Scope<GenericChildProps<T>>) -> Element {
    render! {
        div { "{&cx.props.data}" }
    }
}

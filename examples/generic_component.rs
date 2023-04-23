use std::fmt::Display;

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! { generic_child {
        data: 0i32
    } })
}

#[derive(PartialEq, Props)]
struct GenericChildProps<T: Display + PartialEq> {
    data: T,
}

fn generic_child<T: Display + PartialEq>(cx: Scope<GenericChildProps<T>>) -> Element {
    let data = &cx.props.data;

    cx.render(rsx! { div {
        "{data}"
    } })
}

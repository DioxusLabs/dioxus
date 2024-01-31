use std::fmt::Display;

use dioxus::prelude::*;

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

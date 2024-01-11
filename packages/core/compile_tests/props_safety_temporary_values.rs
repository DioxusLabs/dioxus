use dioxus::prelude::*;

fn main() {}

fn app() -> Element {
    let count = vec![1, 2, 3];

    render! { unsafe_child_component { borrowed: &count } }
}

#[derive(Props)]
struct Testing<'a> {
    borrowed: &'a Vec<u32>,
}

fn unsafe_child_component<'a>(cx: Testing) -> Element {
    render! { div { "{cx.borrowed:?}" } }
}

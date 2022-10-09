#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

// ANCHOR: App
fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        Likes {
            score: 42,
        },
    })
}
// ANCHOR_END: App

// ANCHOR: Likes
// Remember: Owned props must implement `PartialEq`!
#[derive(PartialEq, Props)]
struct LikesProps {
    score: i32,
}

fn Likes(cx: Scope<LikesProps>) -> Element {
    cx.render(rsx! {
        div {
            "This post has ",
            b { "{cx.props.score}" },
            " likes"
        }
    })
}
// ANCHOR_END: Likes

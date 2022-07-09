// ANCHOR: all
#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    // ANCHOR: Clickable_usage
    cx.render(rsx! {
        Clickable {
            href: "https://www.youtube.com/watch?v=C-M2hs3sXGo",
            body: cx.render(rsx!("How to " i {"not"} " be seen")),
        }
    })
    // ANCHOR_END: Clickable_usage
}

// ANCHOR: Clickable
#[derive(Props)]
struct ClickableProps<'a> {
    href: &'a str,
    body: Element<'a>,
}

fn Clickable<'a>(cx: Scope<'a, ClickableProps<'a>>) -> Element {
    cx.render(rsx!(
        a {
            href: "{cx.props.href}",
            class: "fancy-button",
            &cx.props.body
        }
    ))
}
// ANCHOR_END: Clickable

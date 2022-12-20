// ANCHOR: all
#![allow(non_snake_case, unused)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    // ANCHOR: Clickable_usage
    cx.render(rsx! {
        Clickable {
            href: "https://www.youtube.com/watch?v=C-M2hs3sXGo",
            "How to " i {"not"} " be seen"
        }
    })
    // ANCHOR_END: Clickable_usage
}

#[derive(Props)]
struct ClickableProps<'a> {
    href: &'a str,
    children: Element<'a>,
}

// ANCHOR: Clickable
fn Clickable<'a>(cx: Scope<'a, ClickableProps<'a>>) -> Element {
    match cx.props.children {
        Some(VNode { dynamic_nodes, .. }) => {
            todo!("render some stuff")
        }
        _ => {
            todo!("render some other stuff")
        }
    }
}
// ANCHOR_END: Clickable

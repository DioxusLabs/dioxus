#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

#[test]
fn test_borrowed_state() {
    let _ = VirtualDom::new(Parent);
}

fn Parent(cx: Scope) -> Element {
    let value = cx.use_hook(|_| String::new());

    cx.render(rsx! {
        div {
            Child { name: value }
            Child { name: value }
            Child { name: value }
            Child { name: value }
        }
    })
}

#[derive(Props)]
struct ChildProps<'a> {
    name: &'a str,
}

fn Child<'a>(cx: Scope<'a, ChildProps<'a>>) -> Element {
    cx.render(rsx! {
        div {
            h1 { "it's nested" }
            Child2 { name: cx.props.name }
        }
    })
}

#[derive(Props)]
struct Grandchild<'a> {
    name: &'a str,
}

fn Child2<'a>(cx: Scope<'a, Grandchild<'a>>) -> Element {
    cx.render(rsx! {
        div { "Hello {cx.props.name}!" }
    })
}

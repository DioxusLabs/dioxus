#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {
    let mut dom = VirtualDom::new(parent);
    let edits = dom.rebuild();
    dbg!(edits);
}

fn parent(cx: Scope) -> Element {
    let value = cx.use_hook(|_| String::new(), |f| f);

    cx.render(rsx! {
        div {
            child(
                name: value,
                h1 {"hi"}
            )
        }
    })
}

#[derive(Props)]
struct ChildProps<'a> {
    name: &'a str,
    children: Element<'a>,
}

fn child<'a>(cx: Scope<'a, ChildProps<'a>>) -> Element {
    cx.render(rsx! {
        div {
            "it's nested {cx.props.name}"
            {&cx.props.children}
        }
    })
}

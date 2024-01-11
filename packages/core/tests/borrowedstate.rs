#![allow(non_snake_case)]

use dioxus::core::{ElementId, Mutation::*};
use dioxus::prelude::*;

#[test]
fn test_borrowed_state() {
    let mut dom = VirtualDom::new(Parent);

    assert_eq!(
        dom.rebuild_to_vec().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1,) },
            LoadTemplate { name: "template", index: 0, id: ElementId(2,) },
            LoadTemplate { name: "template", index: 0, id: ElementId(3,) },
            HydrateText { path: &[0,], value: "Hello w1!".to_string(), id: ElementId(4,) },
            ReplacePlaceholder { path: &[1,], m: 1 },
            ReplacePlaceholder { path: &[0,], m: 1 },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    );
}

fn Parent(cx: Scope) -> Element {
    let w1 = cx.use_hook(|| String::from("w1"));

    render! {
        div { Child { name: w1 } }
    }
}

#[derive(Props)]
struct ChildProps<'a> {
    name: &'a str,
}

fn Child<'a>(cx: Scope<'a, ChildProps<'a>>) -> Element {
    render! {
        div {
            h1 { "it's nested" }
            Child2 { name: cx.props.name }
        }
    }
}

#[derive(Props)]
struct Grandchild<'a> {
    name: &'a str,
}

fn Child2<'a>(cx: Scope<'a, Grandchild<'a>>) -> Element {
    render!( div { "Hello {cx.props.name}!" } )
}

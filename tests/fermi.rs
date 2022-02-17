#![allow(non_snake_case)]

use dioxus::prelude::*;
use fermi::*;

#[test]
fn test_fermi() {
    let mut app = VirtualDom::new(App);
    app.rebuild();
}

static TITLE: Atom<String> = |_| "".to_string();
static USERS: AtomFamily<u32, String> = |_| Default::default();

fn App(cx: Scope) -> Element {
    cx.render(rsx!(
        Leaf { id: 0 }
        Leaf { id: 1 }
        Leaf { id: 2 }
    ))
}

#[derive(Debug, PartialEq, Props)]
struct LeafProps {
    id: u32,
}

fn Leaf(cx: Scope<LeafProps>) -> Element {
    let _user = use_read(&cx, TITLE);
    let _user = use_read(&cx, USERS);

    rsx!(cx, div {
        button {
            onclick: move |_| {},
            "Start"
        }
        button {
            onclick: move |_| {},
            "Stop"
        }
        button {
            onclick: move |_| {},
            "Reverse"
        }
    })
}

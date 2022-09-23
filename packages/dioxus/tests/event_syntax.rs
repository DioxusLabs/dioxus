#![allow(non_snake_case)]

use dioxus::{events::MouseEvent, prelude::*};

#[test]
fn test_borrowed_state() {
    let _ = VirtualDom::new(Parent);
}

fn Parent(cx: Scope) -> Element {
    let handler = |evt: &MouseEvent| {
        let _r = evt.held_buttons();
    };

    cx.render(rsx! {
        div {
            onclick: handler
        }
    })
}

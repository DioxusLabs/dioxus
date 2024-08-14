#![allow(non_snake_case)]

use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

/// Should push the text node onto the stack and modify it
#[test]
fn many_roots() {
    fn app() -> Element {
        rsx! {
            div {
                MyNav {}
                MyOutlet {}
            }
        }
    }

    fn MyNav() -> Element {
        rsx!(
            div { "trailing nav" }
            div { "whhhhh"}
            div { "bhhhh" }
        )
    }

    fn MyOutlet() -> Element {
        rsx!(
            div { "homepage 1" }
        )
    }

    let mut dom = VirtualDom::new(app);
    let edits = dom.rebuild_to_vec();

    assert_eq!(
        edits.edits,
        [
            // load the div {} container
            LoadTemplate { index: 0, id: ElementId(1) },
            // Load myoutlet first
            LoadTemplate { index: 0, id: ElementId(2) },
            ReplacePlaceholder { path: &[1], m: 1 },
            // Then the MyNav
            LoadTemplate { index: 0, id: ElementId(3) },
            LoadTemplate { index: 1, id: ElementId(4) },
            LoadTemplate { index: 2, id: ElementId(5) },
            ReplacePlaceholder { path: &[0], m: 3 },
            // Then mount the div to the dom
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    )
}

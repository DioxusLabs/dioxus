#![allow(non_snake_case)]

use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::{AttributeValue, ElementId};
use pretty_assertions::assert_eq;

/// Should push the text node onto the stack and modify it
/// Regression test for https://github.com/DioxusLabs/dioxus/issues/2809 and https://github.com/DioxusLabs/dioxus/issues/3055
#[test]
fn many_roots() {
    fn app() -> Element {
        let width = "100%";
        rsx! {
            div {
                MyNav {}
                MyOutlet {}
                div {
                    // We need to make sure that dynamic attributes are set before the nodes before them are expanded
                    // If they are set after, then the paths are incorrect
                    width,
                }
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
            // Set the width attribute first
            AssignId { path: &[2], id: ElementId(2,) },
            SetAttribute {
                name: "width",
                ns: Some("style",),
                value: AttributeValue::Text("100%".to_string()),
                id: ElementId(2,),
            },
            // Load MyOutlet next
            LoadTemplate { index: 0, id: ElementId(3) },
            ReplacePlaceholder { path: &[1], m: 1 },
            // Then MyNav
            LoadTemplate { index: 0, id: ElementId(4) },
            LoadTemplate { index: 1, id: ElementId(5) },
            LoadTemplate { index: 2, id: ElementId(6) },
            ReplacePlaceholder { path: &[0], m: 3 },
            // Then mount the div to the dom
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    )
}

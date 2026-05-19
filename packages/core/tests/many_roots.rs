#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

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

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                div {
                    div { "trailing nav" }
                    div { "whhhhh" }
                    div { "bhhhh" }
                    div { "homepage 1" }
                    div { width: "100%" }
                }
            },
        )
        .assert_edit_summary(0, |s| assert_eq!(s.set_attrs, 1))
        .run();
}

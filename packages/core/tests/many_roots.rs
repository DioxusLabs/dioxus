#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;

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

    fn expected() -> Element {
        rsx! {
            div {
                div { "trailing nav" }
                div { "whhhhh" }
                div { "bhhhh" }
                div { "homepage 1" }
                div { width: "100%" }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    let summary = oracle.rebuild(&mut dom);

    oracle.assert_matches(expected);
    assert_eq!(summary.set_attrs, 1);
}

#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_renderer_oracle::{RendererOracle, SnapshotNode};

macro_rules! oversized_static_divs {
    ($render:ident) => {
        $render! {
            div { "0" }
            div { "1" }
            div { "2" }
            div { "3" }
            div { "4" }
            div { "5" }
            div { "6" }
            div { "7" }
            div { "8" }
            div { "9" }
            div { "10" }
            div { "11" }
            div { "12" }
            div { "13" }
            div { "14" }
            div { "15" }
            div { "16" }
            div { "17" }
            div { "18" }
            div { "19" }
            div { "20" }
            div { "21" }
            div { "22" }
            div { "23" }
            div { "24" }
            div { "25" }
            div { "26" }
            div { "27" }
            div { "28" }
            div { "29" }
            div { "30" }
            div { "31" }
            div { "32" }
            div { "33" }
            div { "34" }
            div { "35" }
            div { "36" }
            div { "37" }
            div { "38" }
            div { "39" }
            div { "40" }
            div { "41" }
            div { "42" }
            div { "43" }
            div { "44" }
            div { "45" }
            div { "46" }
            div { "47" }
            div { "48" }
            div { "49" }
            div { "50" }
            div { "51" }
            div { "52" }
            div { "53" }
            div { "54" }
            div { "55" }
            div { "56" }
            div { "57" }
            div { "58" }
            div { "59" }
            div { "60" }
            div { "61" }
            div { "62" }
            div { "63" }
            div { "64" }
            div { "65" }
            div { "66" }
            div { "67" }
            div { "68" }
            div { "69" }
        }
    };
}

macro_rules! render_as_roots {
    ($($node:tt)*) => {
        rsx! {
            $($node)*
        }
    };
}

macro_rules! render_as_children {
    ($($node:tt)*) => {
        rsx! {
            section {
                $($node)*
            }
        }
    };
}

fn render_app(app: fn() -> Element) -> Vec<SnapshotNode> {
    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    oracle.snapshot()
}

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

#[test]
fn large_static_root_block_renders_through_dynamic_chunks() {
    fn app() -> Element {
        oversized_static_divs!(render_as_roots)
    }

    let snapshot = render_app(app);
    assert_eq!(snapshot.len(), 70);
    assert!(snapshot.iter().all(|node| matches!(
        node,
        SnapshotNode::Element { tag, .. } if tag == "div"
    )));
}

#[test]
fn large_static_child_block_renders_through_dynamic_chunks() {
    fn app() -> Element {
        oversized_static_divs!(render_as_children)
    }

    let snapshot = render_app(app);
    let [SnapshotNode::Element { tag, children, .. }] = snapshot.as_slice() else {
        panic!("expected one section element");
    };
    assert_eq!(tag, "section");
    assert_eq!(children.len(), 70);
    assert!(children.iter().all(|node| matches!(
        node,
        SnapshotNode::Element { tag, .. } if tag == "div"
    )));
}

/// Regression test for deeply nested elements. Nesting deeper than the old
/// 32-level template path-stack cap (but within the splitter's bit-width limit)
/// used to abort macro expansion with an opaque "template path stack capacity
/// exceeded" panic. These 40 levels must now lower and render normally.
#[test]
fn deeply_nested_elements_lower_without_panicking() {
    fn app() -> Element {
        rsx! {
            div { div { div { div { div {
            div { div { div { div { div {
            div { div { div { div { div {
            div { div { div { div { div {
            div { div { div { div { div {
            div { div { div { div { div {
            div { div { div { div { div {
            div { div { div { div { div {
                "deep nesting marker"
            } } } } }
            } } } } }
            } } } } }
            } } } } }
            } } } } }
            } } } } }
            } } } } }
            } } } } }
        }
    }

    let snapshot = render_app(app);
    let mut node = snapshot.first().expect("one root div");
    for _ in 0..40 {
        let SnapshotNode::Element { tag, children, .. } = node else {
            panic!("expected a nested div at every level");
        };
        assert_eq!(tag, "div");
        node = children.first().expect("each div has a single child");
    }
}

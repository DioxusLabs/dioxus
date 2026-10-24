//! Inline `style` attributes are written as a single value that includes both the
//! static template styles and every dynamic style declaration. Replacing the whole
//! attribute is what lets removed declarations actually disappear from the DOM.

use dioxus::prelude::*;
use dioxus_core::{ScopeId, generation};
use dioxus_renderer_oracle::{RendererOracle, SnapshotNode};

fn style_value(nodes: &[SnapshotNode], tag: &str) -> Option<String> {
    for node in nodes {
        if let SnapshotNode::Element { tag: node_tag, attrs, children, .. } = node {
            if node_tag == tag
                && let Some(style) = attrs.iter().find(|attr| attr.name == "style")
            {
                return Some(style.value.clone());
            }
            if let Some(style) = style_value(children, tag) {
                return Some(style);
            }
        }
    }
    None
}

#[test]
fn removed_style_declarations_are_not_restored() {
    // Regression test for https://github.com/DioxusLabs/dioxus/issues/4389.
    fn app() -> Element {
        let style = if generation() == 0 {
            "border-radius: 12px; background-color: red;"
        } else {
            "border-radius: 0px;"
        };

        rsx! {
            button { style, "Toggle Style" }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    assert_eq!(
        style_value(&oracle.snapshot(), "button").as_deref(),
        Some("border-radius: 12px; background-color: red;")
    );

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);

    assert_eq!(
        style_value(&oracle.snapshot(), "button").as_deref(),
        Some("border-radius: 0px;"),
        "the removed background-color declaration must not be restored"
    );
}

#[test]
fn static_and_dynamic_styles_are_merged() {
    fn app() -> Element {
        let background = if generation() == 0 { "red" } else { "blue" };

        rsx! {
            div {
                style: "color: green;",
                background_color: "{background}",
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);

    assert_eq!(
        style_value(&oracle.snapshot(), "div").as_deref(),
        Some("color: green; background-color: red;")
    );

    dom.mark_dirty(ScopeId::APP);
    oracle.render(&mut dom);

    assert_eq!(
        style_value(&oracle.snapshot(), "div").as_deref(),
        Some("color: green; background-color: blue;")
    );
}

use dioxus::prelude::*;
use dioxus_core::VNode;
use dioxus_renderer_oracle::{RendererOracle, SnapshotAttr, SnapshotNode, fresh_snapshot};

dioxus::html::define_elements! {
    #[element(name = "style-panel")]
    stylePanel {}

    #[element(name = "sized-panel")]
    sizedPanel {
        width,
    }
}

fn typed_dynamic_attrs() -> Element {
    let color = "red";
    let width = "320";
    let value = "hello";
    let selected = true;

    rsx! {
        div {
            background_color: "{color}",
        }
        img {
            width: "{width}",
        }
        input {
            value: "{value}",
        }
        option {
            selected: selected,
        }
    }
}

fn mixed_dynamic_attr_and_child() -> Element {
    let class = "primary";
    let label = "hello";

    rsx! {
        div {
            class: "{class}",
            "{label}"
        }
    }
}

#[allow(non_snake_case)]
fn EmptyHeadLikeComponent() -> Element {
    VNode::empty()
}

fn root_dynamic_before_static_root_with_nested_dynamic_attr() -> Element {
    let class = "body";

    rsx! {
        EmptyHeadLikeComponent {}
        section {
            span {
                class: "{class}",
                "body"
            }
        }
    }
}

fn custom_element_gated_attrs() -> Element {
    let css_width = "12px";
    let element_width = "7";

    rsx! {
        stylePanel {
            width: "{css_width}",
        }
        sizedPanel {
            width: "{element_width}",
        }
    }
}

#[test]
fn typed_dynamic_attr_metadata_survives_direct_rsx_codegen() {
    let mut dom = VirtualDom::new(typed_dynamic_attrs);
    let mut oracle = RendererOracle::new();
    let summary = oracle.rebuild(&mut dom);

    oracle.assert_matches(typed_dynamic_attrs);
    assert_eq!(summary.set_attrs, 4);
}

#[test]
fn custom_elements_get_gated_global_attrs_unless_they_define_the_attr() {
    let snapshot = fresh_snapshot(custom_element_gated_attrs);

    assert_eq!(
        snapshot,
        vec![
            SnapshotNode::Element {
                tag: "style-panel".to_string(),
                namespace: None,
                attrs: vec![SnapshotAttr {
                    name: "width".to_string(),
                    namespace: Some("style".to_string()),
                    value: "12px".to_string(),
                }],
                listeners: Vec::new(),
                children: Vec::new(),
            },
            SnapshotNode::Element {
                tag: "sized-panel".to_string(),
                namespace: None,
                attrs: vec![SnapshotAttr {
                    name: "width".to_string(),
                    namespace: None,
                    value: "7".to_string(),
                }],
                listeners: Vec::new(),
                children: Vec::new(),
            },
        ]
    );
}

#[test]
fn dynamic_attr_and_child_share_one_anchor() {
    let vnode = mixed_dynamic_attr_and_child().unwrap();
    let div = vnode
        .template
        .root_slots()
        .find_map(|(_, op, _)| op)
        .expect("expected a static root element");
    let anchors = vnode
        .template
        .element_dynamic_anchors(div)
        .collect::<Vec<_>>();

    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].value_count(), 2);
    assert_eq!(vnode.dynamic_attr_indices_for_anchor(anchors[0]).count(), 1);
    assert_eq!(vnode.dynamic_node_indices_for_anchor(anchors[0]).count(), 1);

    let mut dom = VirtualDom::new(mixed_dynamic_attr_and_child);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(mixed_dynamic_attr_and_child);
}

#[test]
fn nested_dynamic_attr_after_root_dynamic_uses_static_root_slot() {
    let mut dom = VirtualDom::new(root_dynamic_before_static_root_with_nested_dynamic_attr);
    let mut oracle = RendererOracle::new();

    oracle.rebuild(&mut dom);
    oracle.assert_matches(root_dynamic_before_static_root_with_nested_dynamic_attr);
}

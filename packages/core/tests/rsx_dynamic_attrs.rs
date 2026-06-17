use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;

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

#[test]
fn typed_dynamic_attr_metadata_survives_direct_rsx_codegen() {
    let mut dom = VirtualDom::new(typed_dynamic_attrs);
    let mut oracle = RendererOracle::new();
    let summary = oracle.rebuild(&mut dom);

    oracle.assert_matches(typed_dynamic_attrs);
    assert_eq!(summary.set_attrs, 4);
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

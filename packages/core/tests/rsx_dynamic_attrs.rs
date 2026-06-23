use dioxus::prelude::*;
use dioxus_core::{Mutation, Mutations, ScopeId, VNode, VNodeChild};
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

fn dynamic_attr_and_trailing_dynamic_child() -> Element {
    let id = "outer";
    let label = "before";

    rsx! {
        div {
            id: "{id}",
            "{label}"
            button { "middle" }
            TailComponent {
                id: 7
            }
        }
    }
}

#[component]
fn TailComponent(id: i32) -> Element {
    rsx! {
        span {
            "{id}"
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

fn separated_empty_fragment_slots() -> Element {
    let show_a = false;
    let show_b = false;

    rsx! {
        div {
            if show_a {
                "A"
            }
            span { "S" }
            if show_b {
                "B"
            }
        }
    }
}

fn root_element(vnode: &VNode) -> dioxus_core::StaticElement<'_> {
    vnode
        .children()
        .find_map(|child| match child {
            VNodeChild::Element(element) => Some(element),
            _ => None,
        })
        .expect("expected a static root element")
}

static SHOW_SEPARATED_SLOT_B: GlobalSignal<bool> = Signal::global(|| false);
static SHOW_SEPARATED_SLOT_A: GlobalSignal<bool> = Signal::global(|| false);

fn separated_empty_fragment_slots_dynamic_app() -> Element {
    rsx! {
        SeparatedEmptyFragmentSlotsDynamicChild {}
    }
}

#[component]
fn SeparatedEmptyFragmentSlotsDynamicChild() -> Element {
    let show_a = SHOW_SEPARATED_SLOT_A();
    let show_b = SHOW_SEPARATED_SLOT_B();

    rsx! {
        div {
            if show_a {
                "A"
            }
            span { "S" }
            if show_b {
                "B"
            }
        }
        button { "fill b" }
        button { "fill a" }
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
fn dynamic_attr_and_child_share_static_path() {
    let vnode = mixed_dynamic_attr_and_child().unwrap();
    let div = root_element(&vnode);
    let anchors = vnode
        .dynamic_anchors()
        .filter(|anchor| anchor.parent_element_op_index() == Some(div.op()))
        .collect::<Vec<_>>();
    let attr_anchor = anchors
        .iter()
        .copied()
        .find(|anchor| anchor.attrs().len() > 0)
        .expect("dynamic attr anchor");
    let node_anchor = anchors
        .iter()
        .copied()
        .find(|anchor| anchor.nodes().len() > 0)
        .expect("dynamic node anchor");

    assert_eq!(attr_anchor.attrs().len(), 1);
    assert_eq!(node_anchor.nodes().len(), 1);
    assert!(attr_anchor.static_path() == node_anchor.static_path());
    assert!(!attr_anchor.is_last_static_node());
    assert!(node_anchor.is_last_static_node());

    let mut dom = VirtualDom::new(mixed_dynamic_attr_and_child);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(mixed_dynamic_attr_and_child);
}

#[test]
fn dynamic_attr_and_trailing_dynamic_child_uses_split_anchor_ranges() {
    let vnode = dynamic_attr_and_trailing_dynamic_child().unwrap();
    let div = root_element(&vnode);
    let anchors = vnode
        .dynamic_anchors()
        .filter(|anchor| anchor.parent_element_op_index() == Some(div.op()))
        .collect::<Vec<_>>();
    let attr_anchors = anchors
        .iter()
        .copied()
        .filter(|anchor| anchor.attrs().len() > 0)
        .collect::<Vec<_>>();
    let node_anchors = anchors
        .iter()
        .copied()
        .filter(|anchor| anchor.nodes().len() > 0)
        .collect::<Vec<_>>();

    assert_eq!(attr_anchors.len(), 1);
    assert_eq!(attr_anchors[0].attrs().len(), 1);
    assert_eq!(node_anchors.len(), 2);
    assert_eq!(
        node_anchors
            .iter()
            .map(|anchor| anchor.nodes().len())
            .sum::<usize>(),
        2
    );

    let mut dom = VirtualDom::new(dynamic_attr_and_trailing_dynamic_child);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(dynamic_attr_and_trailing_dynamic_child);
}

#[test]
fn nested_dynamic_attr_after_root_dynamic_uses_static_root_slot() {
    let mut dom = VirtualDom::new(root_dynamic_before_static_root_with_nested_dynamic_attr);
    let mut oracle = RendererOracle::new();

    oracle.rebuild(&mut dom);
    oracle.assert_matches(root_dynamic_before_static_root_with_nested_dynamic_attr);
}

#[test]
fn separated_empty_fragment_slots_stay_inside_static_parent() {
    let vnode = separated_empty_fragment_slots().unwrap();
    let div = root_element(&vnode);
    let anchors = vnode
        .dynamic_anchors()
        .filter(|anchor| anchor.parent_element_op_index() == Some(div.op()))
        .filter(|anchor| anchor.nodes().len() > 0)
        .collect::<Vec<_>>();

    assert_eq!(anchors.len(), 2);

    let before_span = anchors
        .iter()
        .copied()
        .find(|anchor| !anchor.is_last_static_node())
        .expect("expected leading empty fragment anchor");
    assert!(!before_span.is_last_static_node());

    let after_span = anchors
        .iter()
        .copied()
        .find(|anchor| anchor.is_last_static_node() && !anchor.static_path().is_empty())
        .expect("expected trailing empty fragment anchor");
    assert!(after_span.is_last_static_node());
}

#[test]
fn no_op_rebuild_places_separated_empty_fragment_inside_static_parent() {
    let mut dom = VirtualDom::new(separated_empty_fragment_slots_dynamic_app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    dom.in_scope(ScopeId::APP, || *SHOW_SEPARATED_SLOT_B.write() = true);

    let mut mutations = Mutations::default();
    dom.render_immediate(&mut mutations);

    let mut stack = vec![Some(dioxus_core::ElementId::ROOT)];
    let mut root_appends = false;
    for mutation in &mutations.edits {
        match mutation {
            Mutation::PushId { id } => stack.push(Some(*id)),
            Mutation::PopId { .. } | Mutation::Pop => {
                stack.pop();
            }
            Mutation::CreateElement { .. } | Mutation::CreateText { .. } => stack.push(None),
            Mutation::AppendChildren { m } => {
                let parent = stack[stack.len() - *m - 1];
                root_appends |= parent == Some(dioxus_core::ElementId::ROOT);
                stack.truncate(stack.len() - *m);
            }
            Mutation::ReplaceWith { m } => {
                let target = stack.len() - *m - 1;
                stack.truncate(target);
                for _ in 0..*m {
                    stack.push(None);
                }
            }
            Mutation::InsertAfter { m } | Mutation::InsertBefore { m } => {
                stack.truncate(stack.len() - *m);
            }
            Mutation::Child { .. }
            | Mutation::Clone
            | Mutation::SetAttribute { .. }
            | Mutation::SetText { .. }
            | Mutation::NewEventListener { .. }
            | Mutation::RemoveEventListener { .. }
            | Mutation::Remove => {}
        }
    }
    assert!(
        !root_appends,
        "empty fragment should be inserted into its static parent, not the renderer root: {:#?}",
        mutations.edits
    );
}

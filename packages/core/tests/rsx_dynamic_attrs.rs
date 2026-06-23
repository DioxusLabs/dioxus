use dioxus::prelude::*;
use dioxus_core::{Mutation, Mutations, ScopeId, VNode, VNodeChild};
use dioxus_core_template::TemplateSlotTarget;
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
fn dynamic_attr_and_child_share_one_anchor() {
    let vnode = mixed_dynamic_attr_and_child().unwrap();
    let div = root_element(&vnode);
    let attr_groups = div.dynamic_attributes().collect::<Vec<_>>();
    let node_groups = vnode
        .dynamic_nodes()
        .filter(|group| group.parent_element_op_index() == Some(div.op()))
        .collect::<Vec<_>>();

    assert_eq!(attr_groups.len(), 1);
    assert_eq!(node_groups.len(), 1);
    assert_eq!(attr_groups[0].anchor_index(), node_groups[0].anchor_index());
    assert_eq!(attr_groups[0].attrs().count(), 1);
    assert_eq!(node_groups[0].attrs().count(), 1);

    let mut dom = VirtualDom::new(mixed_dynamic_attr_and_child);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(mixed_dynamic_attr_and_child);
}

#[test]
fn dynamic_attr_and_trailing_dynamic_child_follow_value_order() {
    let vnode = dynamic_attr_and_trailing_dynamic_child().unwrap();
    let div = root_element(&vnode);
    let attr_groups = div.dynamic_attributes().collect::<Vec<_>>();
    let mut children_iter = div.children();
    assert_eq!(children_iter.len(), 3);
    let children = children_iter.by_ref().collect::<Vec<_>>();
    assert_eq!(children_iter.len(), 0);

    assert_eq!(attr_groups.len(), 1);
    assert_eq!(children.len(), 3);
    let attr_group = attr_groups[0];
    let VNodeChild::Dynamic(before_button_group) = children[0] else {
        panic!("expected leading dynamic child first");
    };
    let VNodeChild::Dynamic(append_group) = children[2] else {
        panic!("expected trailing dynamic child third");
    };

    assert!(attr_group.ids().eq([0]));
    assert!(before_button_group.ids().eq([1]));
    assert!(append_group.ids().eq([2]));
    assert!(matches!(
        before_button_group.slot_target(),
        TemplateSlotTarget::BeforeStatic(_)
    ));
    assert!(matches!(
        append_group.slot_target(),
        TemplateSlotTarget::AppendChildren(path) if !path.is_empty()
    ));

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
    let groups = vnode
        .dynamic_nodes()
        .filter(|group| group.parent_element_op_index() == Some(div.op()))
        .collect::<Vec<_>>();

    assert_eq!(groups.len(), 2);
    for group in &groups {
        assert!(group.parent_element_op_index().is_some());
    }

    let before_span = groups
        .iter()
        .copied()
        .find(|group| group.ids().eq([0]))
        .expect("expected leading empty fragment anchor");
    assert!(matches!(
        before_span.slot_target(),
        TemplateSlotTarget::BeforeStatic(_)
    ));

    let after_span = groups
        .iter()
        .copied()
        .find(|group| group.ids().eq([1]))
        .expect("expected trailing empty fragment anchor");
    assert!(matches!(
        after_span.slot_target(),
        TemplateSlotTarget::AppendChildren(path) if !path.is_empty()
    ));
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

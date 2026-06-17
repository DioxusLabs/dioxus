use crate::renderer::RendererOracle;
use crate::snapshot::{SnapshotAttr, SnapshotNode, attr_key, attr_to_string};
use dioxus_core::{Attribute, AttributeValue, DynamicNode, Element, MountedVNode, VirtualDom};
use std::any::Any;

/// Render `app` from scratch into a stable snapshot.
pub fn fresh_snapshot(app: fn() -> Element) -> Vec<SnapshotNode> {
    let mut vdom = VirtualDom::new(app);
    let mut renderer = RendererOracle::new();
    renderer.rebuild(&mut vdom);
    renderer.assert_matches_vdom(&vdom);
    renderer.snapshot()
}

/// Snapshot the raw rendered VDOM tree without using renderer mutations.
pub fn vdom_snapshot(vdom: &VirtualDom) -> Vec<SnapshotNode> {
    vnode_snapshot(vdom, vdom.base_scope().try_mounted_root_node().unwrap())
}

/// Render pending work from `vdom` into `renderer` and return the resulting snapshot.
pub fn render_immediate_snapshot(
    vdom: &mut VirtualDom,
    renderer: &mut RendererOracle,
) -> Vec<SnapshotNode> {
    renderer.render(vdom);
    renderer.assert_matches_vdom(vdom);
    renderer.snapshot()
}

/// Render pending work from `vdom` into `renderer` and assert it matches a fresh rebuild of `app`.
pub fn assert_immediate_matches_fresh(
    vdom: &mut VirtualDom,
    renderer: &mut RendererOracle,
    app: fn() -> Element,
) {
    let incremental = render_immediate_snapshot(vdom, renderer);
    let fresh = fresh_snapshot(app);
    pretty_assertions::assert_eq!(
        incremental,
        fresh,
        "incremental render diverged from a fresh rebuild"
    );
}

/// Assert that rendering `app` from scratch matches `expected`.
pub fn assert_fresh_snapshot_eq(app: fn() -> Element, expected: &[SnapshotNode]) {
    let actual = fresh_snapshot(app);
    pretty_assertions::assert_eq!(
        actual,
        expected,
        "fresh render snapshot diverged from expected tree"
    );
}

/// Assert that an immediate render emits no Dioxus mutations.
pub fn assert_no_mutations(vdom: &mut VirtualDom) {
    let mutations = vdom.render_immediate_to_vec();
    assert!(
        mutations.edits.is_empty(),
        "expected no mutations, got {} mutation(s):\n{:#?}",
        mutations.edits.len(),
        mutations.edits
    );
}

fn vnode_snapshot(vdom: &VirtualDom, vnode: MountedVNode<'_>) -> Vec<SnapshotNode> {
    let mut out = Vec::new();
    for (_, static_op, dynamic_anchor) in vnode.template.root_slots() {
        if let Some(anchor) = dynamic_anchor {
            for index in vnode.dynamic_node_indices_for_anchor(anchor) {
                out.extend(dynamic_node_snapshot(vdom, vnode, index));
            }
            continue;
        }

        let op = static_op.expect("root slot must be static or dynamic");
        out.extend(template_node_snapshot(vdom, vnode, op));
    }
    out
}

fn element_children_snapshot(
    vdom: &VirtualDom,
    vnode: MountedVNode<'_>,
    op: usize,
) -> Vec<SnapshotNode> {
    let mut out = Vec::new();
    let static_children = vnode.template.static_children(op).collect::<Vec<_>>();
    for slot in 0..=static_children.len() {
        for anchor in vnode.dynamic_node_anchors_for_slot(op, slot) {
            for idx in vnode.dynamic_node_indices_for_anchor(anchor) {
                out.extend(dynamic_node_snapshot(vdom, vnode, idx));
            }
        }
        if let Some(&child_op) = static_children.get(slot) {
            out.extend(template_node_snapshot(vdom, vnode, child_op));
        }
    }
    out
}

fn template_node_snapshot(
    vdom: &VirtualDom,
    vnode: MountedVNode<'_>,
    op: usize,
) -> Vec<SnapshotNode> {
    if let Some((tag, namespace)) = vnode.template.element_meta_at_op(op) {
        let mut element_attrs = Vec::new();
        let mut listeners = Vec::new();

        for (name, value, namespace) in vnode.template.static_attrs(op) {
            set_snapshot_attr(
                &mut element_attrs,
                name.to_string(),
                namespace.map(ToString::to_string),
                value.to_string(),
            );
        }

        for anchor in vnode.dynamic_attr_anchors_for_element(op) {
            for id in vnode.dynamic_attr_indices_for_anchor(anchor) {
                let attrs = vnode.dynamic_values[id]
                    .as_attrs()
                    .expect("snapshot attr slot must point at attributes");
                for attr in attrs {
                    apply_dynamic_attr(&mut element_attrs, &mut listeners, attr);
                }
            }
        }

        let rendered_children = element_children_snapshot(vdom, vnode, op);

        vec![SnapshotNode::Element {
            tag: tag.to_string(),
            namespace: namespace.map(ToString::to_string),
            attrs: element_attrs,
            listeners,
            children: rendered_children,
        }]
    } else if let Some(text) = vnode.template.static_text_at_op(op) {
        vec![SnapshotNode::Text(text.to_string())]
    } else {
        unreachable!("snapshot static node must start at a static node op")
    }
}

fn dynamic_node_snapshot(
    vdom: &VirtualDom,
    owner: MountedVNode<'_>,
    id: usize,
) -> Vec<SnapshotNode> {
    match owner.dynamic_values[id]
        .as_node()
        .expect("snapshot node slot must point at a dynamic node")
    {
        DynamicNode::Text(text) => vec![SnapshotNode::Text(text.value.clone())],
        DynamicNode::Fragment(nodes) => {
            let mounted_children = owner.mounted_fragment_children(id, vdom);
            assert_eq!(
                mounted_children.len(),
                nodes.len(),
                "fragment dynamic node {id} is not mounted"
            );
            mounted_children
                .into_iter()
                .flat_map(|node| vnode_snapshot(vdom, node))
                .collect()
        }
        DynamicNode::Component(component) => {
            let scope = component.mounted_scope(id, owner, vdom).unwrap_or_else(|| {
                panic!(
                    "component dynamic node {id} ({}) is not mounted",
                    component.name
                )
            });
            vnode_snapshot(vdom, scope.try_mounted_root_node().unwrap())
        }
    }
}

fn apply_dynamic_attr(
    attrs: &mut Vec<SnapshotAttr>,
    listeners: &mut Vec<String>,
    attr: &Attribute,
) {
    match &attr.value {
        AttributeValue::Listener(_) => {
            let name = attr
                .name
                .strip_prefix("on")
                .unwrap_or(attr.name)
                .to_string();
            match listeners.binary_search(&name) {
                Ok(_) => {}
                Err(index) => listeners.insert(index, name),
            }
        }
        value => match attr_to_string(value) {
            Some(value) => set_snapshot_attr(
                attrs,
                attr.name.to_string(),
                attr.namespace.map(ToString::to_string),
                value,
            ),
            None => remove_snapshot_attr(attrs, attr.name, attr.namespace),
        },
    }
}

fn set_snapshot_attr(
    attrs: &mut Vec<SnapshotAttr>,
    name: String,
    namespace: Option<String>,
    value: String,
) {
    match attrs.binary_search_by(|attr| attr_key(attr).cmp(&(name.as_str(), namespace.as_deref())))
    {
        Ok(index) => attrs[index].value = value,
        Err(index) => attrs.insert(
            index,
            SnapshotAttr {
                name,
                namespace,
                value,
            },
        ),
    }
}

fn remove_snapshot_attr(attrs: &mut Vec<SnapshotAttr>, name: &str, namespace: Option<&str>) {
    if let Ok(index) = attrs.binary_search_by(|attr| attr_key(attr).cmp(&(name, namespace))) {
        attrs.remove(index);
    }
}

/// Convert a panic payload into a readable string for fuzzer/test diagnostics.
pub fn panic_message(payload: &Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

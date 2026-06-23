use crate::renderer::RendererOracle;
use crate::snapshot::{SnapshotAttr, SnapshotNode, attr_key, attr_to_string};
use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, Element, MountedVNode, VNodeChild, VirtualDom,
};

/// Render `app` from scratch into a stable snapshot.
pub fn fresh_snapshot(app: fn() -> Element) -> Vec<SnapshotNode> {
    let mut vdom = VirtualDom::new(app);
    let mut renderer = RendererOracle::new();
    renderer.rebuild(&mut vdom);
    renderer.assert_matches_vdom(&vdom);
    renderer.snapshot()
}

/// Snapshot the rendered VDOM tree directly.
pub(crate) fn vdom_snapshot(vdom: &VirtualDom) -> Vec<SnapshotNode> {
    vnode_snapshot(vdom, vdom.base_scope().try_mounted_root_node().unwrap())
}

/// Assert that an immediate render emits no Dioxus mutations.
#[cfg(test)]
pub(crate) fn assert_no_mutations(vdom: &mut VirtualDom) {
    let mutations = vdom.render_immediate_to_vec();
    assert!(
        mutations.edits.is_empty(),
        "expected no mutations, got {} mutation(s):\n{:#?}",
        mutations.edits.len(),
        mutations.edits
    );
}

fn vnode_snapshot(vdom: &VirtualDom, vnode: MountedVNode<'_>) -> Vec<SnapshotNode> {
    vnode
        .vnode()
        .children()
        .flat_map(|child| child_snapshot(vdom, vnode, child))
        .collect()
}

fn child_snapshot<'a>(
    vdom: &VirtualDom,
    vnode: MountedVNode<'a>,
    child: VNodeChild<'a>,
) -> Vec<SnapshotNode> {
    match child {
        VNodeChild::Element(element) => {
            let mut element_attrs = Vec::new();
            let mut listeners = Vec::new();
            for (name, value, namespace) in element.static_attributes() {
                set_snapshot_attr(
                    &mut element_attrs,
                    name.to_string(),
                    namespace.map(ToString::to_string),
                    value.to_string(),
                );
            }
            for group in element.dynamic_attributes() {
                for attr in group.attrs().flatten() {
                    apply_dynamic_attr(&mut element_attrs, &mut listeners, attr);
                }
            }
            let rendered_children = element
                .children()
                .flat_map(|child| child_snapshot(vdom, vnode, child))
                .collect();

            vec![SnapshotNode::Element {
                tag: element.tag().to_string(),
                namespace: element.namespace().map(ToString::to_string),
                attrs: element_attrs,
                listeners,
                children: rendered_children,
            }]
        }
        VNodeChild::Text(text) => vec![SnapshotNode::Text(text.text().to_string())],
        VNodeChild::Dynamic(group) => {
            let mut out = Vec::new();
            for idx in group.ids() {
                out.extend(dynamic_node_snapshot(vdom, vnode, idx));
            }
            out
        }
    }
}

fn dynamic_node_snapshot(
    vdom: &VirtualDom,
    owner: MountedVNode<'_>,
    id: usize,
) -> Vec<SnapshotNode> {
    match &owner.dynamic_node_values()[id] {
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

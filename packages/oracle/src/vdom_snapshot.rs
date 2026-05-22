#[cfg(test)]
use crate::renderer::RendererOracle;
use crate::snapshot::{
    attr_to_string, remove_attr as remove_snapshot_attr, set_attr as set_snapshot_attr,
    snapshot_attrs, snapshot_listeners, SnapshotAttrs, SnapshotListeners, SnapshotNode,
};
#[cfg(test)]
use dioxus_core::Element;
use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, TemplateAttribute, TemplateNode, VNode, VirtualDom,
};

/// Render `app` from scratch into a stable snapshot.
#[cfg(test)]
pub(crate) fn fresh_snapshot(app: fn() -> Element) -> Vec<SnapshotNode> {
    let mut vdom = VirtualDom::new(app);
    let mut renderer = RendererOracle::new();
    vdom.rebuild(&mut renderer);
    renderer.assert_stack_clean();
    pretty_assertions::assert_eq!(renderer.snapshot(), vdom_snapshot(&vdom));
    renderer.snapshot()
}

/// Snapshot the raw rendered VDOM tree without using renderer mutations.
pub(crate) fn vdom_snapshot(vdom: &VirtualDom) -> Vec<SnapshotNode> {
    vnode_snapshot(vdom, vdom.base_scope().root_node())
}

/// Assert that an immediate render emits no Dioxus mutations.
#[cfg(test)]
pub(crate) fn assert_no_mutations(vdom: &mut VirtualDom) {
    use dioxus_core::Mutations;

    let mut mutations = Mutations::default();
    vdom.render_immediate(&mut mutations);
    assert!(
        mutations.edits.is_empty(),
        "expected no mutations, got {} mutation(s):\n{:#?}",
        mutations.edits.len(),
        mutations.edits
    );
}

fn vnode_snapshot(vdom: &VirtualDom, vnode: &VNode) -> Vec<SnapshotNode> {
    let mut out = Vec::new();
    for (root_idx, root) in vnode.template.roots().iter().enumerate() {
        let path = [root_idx as u8];
        out.extend(template_node_snapshot(vdom, vnode, root, &path));
    }
    out
}

fn template_node_snapshot(
    vdom: &VirtualDom,
    vnode: &VNode,
    node: &TemplateNode,
    path: &[u8],
) -> Vec<SnapshotNode> {
    match node {
        TemplateNode::Element {
            tag,
            namespace,
            attrs,
            children,
        } => {
            let mut element_attrs = SnapshotAttrs::default();
            let mut listeners = SnapshotListeners::default();

            for attr in *attrs {
                if let TemplateAttribute::Static {
                    name,
                    value,
                    namespace,
                } = attr
                {
                    set_snapshot_attr(
                        &mut element_attrs,
                        (*name).to_string(),
                        namespace.map(ToString::to_string),
                        (*value).to_string(),
                    );
                }
            }

            for (idx, attr_path) in vnode.template.attr_paths().iter().enumerate() {
                if *attr_path == path {
                    for attr in &*vnode.dynamic_attrs[idx] {
                        apply_dynamic_attr(&mut element_attrs, &mut listeners, attr);
                    }
                }
            }

            let mut rendered_children = Vec::new();
            for (child_idx, child) in children.iter().enumerate() {
                let mut child_path = Vec::with_capacity(path.len() + 1);
                child_path.extend_from_slice(path);
                child_path.push(child_idx as u8);
                rendered_children.extend(template_node_snapshot(vdom, vnode, child, &child_path));
            }

            vec![SnapshotNode::Element {
                tag: (*tag).to_string(),
                namespace: namespace.map(ToString::to_string),
                attrs: snapshot_attrs(&element_attrs),
                listeners: snapshot_listeners(&listeners),
                children: rendered_children,
            }]
        }
        TemplateNode::Text { text } => vec![SnapshotNode::Text((*text).to_string())],
        TemplateNode::Dynamic { id } => dynamic_node_snapshot(vdom, vnode, *id),
    }
}

fn dynamic_node_snapshot(vdom: &VirtualDom, owner: &VNode, id: usize) -> Vec<SnapshotNode> {
    match &owner.dynamic_nodes[id] {
        DynamicNode::Text(text) => vec![SnapshotNode::Text(text.value.clone())],
        DynamicNode::Fragment(nodes) => nodes
            .iter()
            .flat_map(|node| vnode_snapshot(vdom, node))
            .collect(),
        DynamicNode::Component(component) => {
            let scope = component.mounted_scope(id, owner, vdom).unwrap_or_else(|| {
                panic!(
                    "component dynamic node {id} ({}) is not mounted",
                    component.name
                )
            });
            vnode_snapshot(vdom, scope.root_node())
        }
        DynamicNode::Placeholder(_) => Vec::new(),
    }
}

fn apply_dynamic_attr(
    attrs: &mut SnapshotAttrs,
    listeners: &mut SnapshotListeners,
    attr: &Attribute,
) {
    match &attr.value {
        AttributeValue::Listener(_) => {
            let name = attr
                .name
                .strip_prefix("on")
                .unwrap_or(attr.name)
                .to_string();
            listeners.insert(name);
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

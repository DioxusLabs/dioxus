use crate::renderer::RendererOracle;
use crate::snapshot::{SnapshotAttr, SnapshotNode, attr_key, attr_to_string};
use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, Element, TemplateAttribute, TemplateNode, VNode,
    VirtualDom,
};
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
    vnode_snapshot(vdom, vdom.base_scope().root_node())
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

fn vnode_snapshot(vdom: &VirtualDom, vnode: &VNode) -> Vec<SnapshotNode> {
    template_children_snapshot(vdom, vnode, &[], vnode.template.roots())
}

fn template_children_snapshot(
    vdom: &VirtualDom,
    vnode: &VNode,
    parent_cursor: &[u8],
    children: &[TemplateNode],
) -> Vec<SnapshotNode> {
    let mut out = Vec::new();
    for child_idx in 0..=children.len() {
        for (dynamic_idx, _) in vnode
            .template
            .node_cursors()
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, cursor)| {
                let (slot_parent, insertion_index) = cursor.split_slot();
                slot_parent == parent_cursor && insertion_index == child_idx
            })
        {
            out.extend(dynamic_node_snapshot(vdom, vnode, dynamic_idx));
        }

        if let Some(child) = children.get(child_idx) {
            let mut child_cursor = parent_cursor.to_vec();
            child_cursor.push(child_idx as u8);
            out.extend(template_node_snapshot(vdom, vnode, child, &child_cursor));
        }
    }
    out
}

fn template_node_snapshot(
    vdom: &VirtualDom,
    vnode: &VNode,
    node: &TemplateNode,
    cursor: &[u8],
) -> Vec<SnapshotNode> {
    match node {
        TemplateNode::Element {
            tag,
            namespace,
            attrs,
            children,
        } => {
            let mut element_attrs = Vec::new();
            let mut listeners = Vec::new();

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

            for (idx, attr_cursor) in vnode.template.attr_cursors().iter().enumerate() {
                if attr_cursor.as_slice() == cursor {
                    for attr in &*vnode.dynamic_attrs[idx] {
                        apply_dynamic_attr(&mut element_attrs, &mut listeners, attr);
                    }
                }
            }

            let rendered_children = template_children_snapshot(vdom, vnode, cursor, children);

            vec![SnapshotNode::Element {
                tag: (*tag).to_string(),
                namespace: namespace.map(ToString::to_string),
                attrs: element_attrs,
                listeners,
                children: rendered_children,
            }]
        }
        TemplateNode::Text { text } => vec![SnapshotNode::Text((*text).to_string())],
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

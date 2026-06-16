use crate::renderer::RendererOracle;
use crate::snapshot::{SnapshotAttr, SnapshotNode, attr_key, attr_to_string};
use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, Element, Template, TemplatePath, VNode, VirtualDom,
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
    template_children_snapshot(
        vdom,
        vnode,
        TemplatePath::empty(),
        None,
        vnode.template.root_count(),
    )
}

fn template_children_snapshot(
    vdom: &VirtualDom,
    vnode: &VNode,
    parent_cursor: TemplatePath,
    parent_op: Option<usize>,
    static_count: usize,
) -> Vec<SnapshotNode> {
    let mut out = Vec::new();
    for child_idx in 0..=static_count {
        for (dynamic_idx, _) in vnode.template.node_paths().filter(|(_, cursor)| {
            let (slot_parent, insertion_index) = cursor.split_slot();
            slot_parent == parent_cursor && insertion_index == child_idx
        }) {
            out.extend(dynamic_node_snapshot(vdom, vnode, dynamic_idx));
        }

        let child_op = match parent_op {
            Some(parent_op) => vnode.template.static_child_op(parent_op, child_idx),
            None => vnode.template.root_op_index(child_idx),
        };
        if let Some(child_op) = child_op {
            let child_cursor = child_path(parent_cursor, child_idx);
            out.extend(template_node_snapshot(vdom, vnode, child_op, &child_cursor));
        }
    }
    out
}

fn child_path(parent: TemplatePath, child_idx: usize) -> TemplatePath {
    let mut path = parent.next_child();
    for _ in 0..child_idx {
        path = path.next_sibling();
    }
    path
}

fn template_node_snapshot(
    vdom: &VirtualDom,
    vnode: &VNode,
    op: usize,
    cursor: &TemplatePath,
) -> Vec<SnapshotNode> {
    if let Some((tag, namespace)) = vnode.template.element_meta_at_op(op) {
        let mut element_attrs = Vec::new();
        let mut listeners = Vec::new();

        for attr in template_attributes(vnode.template, op) {
            match attr {
                TemplateAttrView::Static {
                    name,
                    value,
                    namespace,
                } => {
                    set_snapshot_attr(
                        &mut element_attrs,
                        name.to_string(),
                        namespace.map(ToString::to_string),
                        value.to_string(),
                    );
                }
                TemplateAttrView::Dynamic { id } => {
                    let attrs = vnode.dynamic_values[id]
                        .as_attrs()
                        .expect("snapshot attr slot must point at attributes");
                    for attr in attrs {
                        apply_dynamic_attr(&mut element_attrs, &mut listeners, attr);
                    }
                }
            }
        }

        let rendered_children = template_children_snapshot(
            vdom,
            vnode,
            *cursor,
            Some(op),
            static_child_count(vnode.template, op),
        );

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

fn dynamic_node_snapshot(vdom: &VirtualDom, owner: &VNode, id: usize) -> Vec<SnapshotNode> {
    match owner.dynamic_values[id]
        .as_node()
        .expect("snapshot node slot must point at a dynamic node")
    {
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

enum TemplateAttrView<'a> {
    Static {
        name: &'a str,
        value: &'a str,
        namespace: Option<&'a str>,
    },
    Dynamic {
        id: usize,
    },
}

fn template_attributes(
    template: Template,
    element_op: usize,
) -> impl Iterator<Item = TemplateAttrView<'static>> {
    let mut cursor = template
        .element_children_start(element_op)
        .expect("template attr scan must start at an element");
    let end = template
        .first_child_node_op(element_op)
        .expect("template attr scan must start at an element");
    std::iter::from_fn(move || {
        while cursor < end {
            if let Some((name, value, namespace)) = template.static_attr_at_op(cursor) {
                let attr = TemplateAttrView::Static {
                    name,
                    value,
                    namespace,
                };
                cursor += template
                    .attr_op_len(cursor)
                    .expect("static attr op must include metadata");
                return Some(attr);
            }
            if template.dynamic_op_is_attr(cursor) {
                let id = template
                    .dynamic_index_at_op(cursor)
                    .expect("dynamic attr op must have metadata");
                cursor += 1;
                return Some(TemplateAttrView::Dynamic { id });
            }
            return None;
        }
        None
    })
}

fn static_child_count(template: Template, element_op: usize) -> usize {
    let Some(mut cursor) = template.first_child_node_op(element_op) else {
        return 0;
    };
    let Some(end) = template.element_end(element_op) else {
        return 0;
    };

    let mut count = 0;
    while cursor < end {
        if template.is_static_node_op(cursor) {
            count += 1;
            cursor = template.next_sibling_op(cursor);
        } else if template.is_dynamic_node_marker(cursor) {
            cursor = template.next_sibling_op(cursor);
        } else {
            cursor += 1;
        }
    }
    count
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

use crate::snapshot::{SnapshotAttr, SnapshotNode, attr_key, attr_to_string};
use crate::vdom_snapshot::vdom_snapshot;
use dioxus_core::{
    AttributeValue, Element, ElementId, Template, TemplateAttribute, TemplateNode, VirtualDom,
    WriteMutations,
};
use std::fmt;

type NodeId = usize;

/// A stable identity token for a node in the oracle's arena. The same node retains
/// the same token across renders, which lets tests verify that the renderer moved a
/// DOM node (preserving its browser-side state — animations, focus, selection) instead
/// of dropping and re-creating it. Recreated nodes get a fresh `OracleNodeId`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OracleNodeId(usize);

#[derive(Clone, Debug)]
enum NodeKind {
    Document,
    Element {
        tag: String,
        namespace: Option<String>,
    },
    Placeholder,
    Text(String),
}

#[derive(Clone, Debug)]
struct Node {
    kind: NodeKind,
    attrs: Vec<SnapshotAttr>,
    listeners: Vec<String>,
    children: Vec<NodeId>,
    parent: Option<NodeId>,
}

/// A category-level summary of edits applied to the renderer in one render pass.
///
/// Counts edits by *kind* (load template, create text, move, set attribute, ...)
/// without exposing any specific `ElementId` or edit ordering. Tests use this to
/// assert structural properties of the diff that final-DOM snapshots cannot
/// observe — e.g. "this keyed reorder moved at most one node," "this rerender
/// patched text in place without recreating elements," "exactly two attributes
/// changed."
///
/// The summary captures only the most recent render call. It is reset at the
/// start of every `rebuild` / `render` / `wait_and_render`.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct EditSummary {
    /// `load_template` calls — a fresh element subtree was created from a template.
    pub loads: usize,
    /// `create_text_node` calls.
    pub create_texts: usize,
    /// `remove_node` calls.
    pub removes: usize,
    /// `replace_node_with` calls.
    pub replaces: usize,
    /// All four `insert_*` / `append_children` calls — placing nodes into the tree.
    pub inserts: usize,
    /// `push_root` calls — proxy for "an existing live node was brought onto the
    /// stack to be moved." A keyed reorder that moves N survivors emits N pushes.
    pub pushes: usize,
    /// `set_attribute` calls.
    pub set_attrs: usize,
    /// `set_node_text` calls — in-place text patches.
    pub set_texts: usize,
}

impl EditSummary {
    /// Total node-creation operations (`loads + create_texts`).
    pub fn creates(&self) -> usize {
        self.loads + self.create_texts
    }
}

/// An event listener target that has been attached during this renderer's lifetime.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EventListenerTarget {
    pub name: &'static str,
    pub id: ElementId,
}

/// A fast mock renderer that applies Dioxus mutations into an in-memory tree.
pub struct RendererOracle {
    arena: Vec<Option<Node>>,
    element_to_node: Vec<Option<NodeId>>,
    node_to_elements: Vec<Vec<ElementId>>,
    stack: Vec<NodeId>,
    popped_nodes: Vec<NodeId>,
    root: NodeId,
    edit_counters: EditSummary,
    historical_event_listener_targets: Vec<EventListenerTarget>,
}

impl Default for RendererOracle {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererOracle {
    /// Create an empty document with `ElementId(0)` mapped to the document root.
    pub fn new() -> Self {
        let root = 0;
        Self {
            arena: vec![Some(Node {
                kind: NodeKind::Document,
                attrs: Vec::new(),
                listeners: Vec::new(),
                children: Vec::new(),
                parent: None,
            })],
            element_to_node: vec![Some(root)],
            node_to_elements: vec![vec![ElementId(0)]],
            stack: vec![root],
            popped_nodes: Vec::new(),
            root,
            edit_counters: EditSummary::default(),
            historical_event_listener_targets: Vec::new(),
        }
    }

    /// Return a category-level summary of the edits applied during the most
    /// recent `rebuild` / `render` / `wait_and_render` call. See [`EditSummary`].
    pub fn last_edit_summary(&self) -> EditSummary {
        self.edit_counters
    }

    /// Return every event listener target attached since the last clear/rebuild.
    pub fn historical_event_listener_targets(&self) -> &[EventListenerTarget] {
        &self.historical_event_listener_targets
    }

    /// Remove all nodes and reset the renderer to an empty document.
    fn clear(&mut self) {
        *self = Self::new();
    }

    /// Return a stable snapshot of the document root's children.
    pub fn snapshot(&self) -> Vec<SnapshotNode> {
        self.node(self.root)
            .children
            .iter()
            .filter_map(|&child| self.snapshot_node(child))
            .collect()
    }

    /// Return true if two oracle DOMs have the same visible snapshot tree.
    ///
    /// This is equivalent to comparing [`RendererOracle::snapshot`] output, but it
    /// avoids allocating and cloning the full snapshot on the success path.
    pub fn snapshot_eq(&self, other: &Self) -> bool {
        self.visible_children_eq(self.root, other, other.root)
    }

    /// Return the number of non-document nodes currently left on the mutation stack.
    fn pending_stack_nodes(&self) -> usize {
        self.stack.len().saturating_sub(1)
    }

    /// Return true when no mutation-created nodes are left on the stack.
    fn is_stack_clean(&self) -> bool {
        self.stack == [self.root]
    }

    /// Assert that the mutation stack only contains the document root.
    pub(crate) fn assert_stack_clean(&self) {
        if let Err(error) = self.check_stack_clean() {
            panic!("{error}");
        }
    }

    /// Check that the mutation stack only contains the document root.
    pub fn check_stack_clean(&self) -> Result<(), String> {
        if self.is_stack_clean() {
            Ok(())
        } else {
            Err(format!(
                "renderer mutation stack is not clean: expected only document root, got {} extra node(s)",
                self.pending_stack_nodes()
            ))
        }
    }

    /// Rebuild `vdom` into this renderer, assert the renderer stack is clean, and
    /// return the edit summary for the rebuild.
    pub fn rebuild(&mut self, vdom: &mut VirtualDom) -> EditSummary {
        self.clear();
        vdom.rebuild(self);
        self.assert_stack_clean();
        self.edit_counters
    }

    /// Drain pending immediate work from `vdom` into this renderer, assert the
    /// stack is clean, and return the edit summary for the render.
    pub fn render(&mut self, vdom: &mut VirtualDom) -> EditSummary {
        self.edit_counters = EditSummary::default();
        vdom.render_immediate(self);
        self.assert_stack_clean();
        self.edit_counters
    }

    /// Await pending work on `vdom`, then drain it into this renderer.
    pub async fn wait_and_render(&mut self, vdom: &mut VirtualDom) -> EditSummary {
        vdom.wait_for_work().await;
        self.render(vdom)
    }

    /// Find the live [`ElementId`] of the unique element whose tag matches
    /// `tag` (default namespace). Panics if zero or more than one element
    /// matches — tests should make the target unambiguous (add an `id` attr
    /// and use [`Self::element_id_by_attr`] instead when multiple elements
    /// share a tag).
    ///
    /// This is the entry point for firing synthetic events without naming a
    /// specific `ElementId(N)` literal in test code: look up the target
    /// semantically (by tag or by attribute), then pass the returned id to
    /// `vdom.runtime().handle_event(...)`.
    pub fn element_id_by_tag(&self, tag: &str) -> ElementId {
        let mut hits = Vec::new();
        self.collect_element_ids_by_tag(self.root, tag, &mut hits);
        match hits.as_slice() {
            [id] => *id,
            [] => panic!("no live element with tag `{tag}` found in the oracle DOM"),
            many => panic!(
                "tag `{tag}` is ambiguous: {} matching elements (use element_id_by_attr to disambiguate)",
                many.len(),
            ),
        }
    }

    /// Find the live [`ElementId`] of the unique element whose attribute
    /// `attr_name` (in the default namespace) has the value `attr_value`.
    /// Panics if zero or more than one element matches.
    pub fn element_id_by_attr(&self, attr_name: &str, attr_value: &str) -> ElementId {
        let mut hits = Vec::new();
        self.collect_element_ids_by_attr(self.root, attr_name, attr_value, &mut hits);
        match hits.as_slice() {
            [id] => *id,
            [] => panic!("no live element with `{attr_name}={attr_value}` found in the oracle DOM"),
            many => panic!(
                "`{attr_name}={attr_value}` is ambiguous: {} matching elements",
                many.len(),
            ),
        }
    }

    fn collect_element_ids_by_tag(&self, node: NodeId, tag: &str, out: &mut Vec<ElementId>) {
        let n = self.node(node);
        if let NodeKind::Element { tag: t, .. } = &n.kind {
            if t == tag {
                if let Some(id) = self.element_id_for_node(node) {
                    out.push(id);
                }
            }
        }
        for &child in &n.children {
            self.collect_element_ids_by_tag(child, tag, out);
        }
    }

    fn collect_element_ids_by_attr(
        &self,
        node: NodeId,
        attr_name: &str,
        attr_value: &str,
        out: &mut Vec<ElementId>,
    ) {
        let n = self.node(node);
        if let NodeKind::Element { .. } = &n.kind {
            for attr in &n.attrs {
                if attr.name == attr_name && attr.namespace.is_none() && attr.value == attr_value {
                    if let Some(id) = self.element_id_for_node(node) {
                        out.push(id);
                    }
                    break;
                }
            }
        }
        for &child in &n.children {
            self.collect_element_ids_by_attr(child, attr_name, attr_value, out);
        }
    }

    fn element_id_for_node(&self, node: NodeId) -> Option<ElementId> {
        for (idx, mapped) in self.element_to_node.iter().enumerate() {
            if *mapped == Some(node) {
                return Some(ElementId(idx));
            }
        }
        None
    }

    /// Walk the DOM and return `(attr_value, identity)` pairs for every element
    /// carrying an attribute named `attr_name` in the default namespace.
    ///
    /// The identity is stable across renders: a node whose `OracleNodeId` matches
    /// across two snapshots is *the same DOM node*, not a structurally equivalent
    /// re-creation. This is how tests assert that a keyed diff moved nodes instead
    /// of dropping and re-allocating them.
    pub fn identities_by_attr(&self, attr_name: &str) -> Vec<(String, OracleNodeId)> {
        let mut out = Vec::new();
        self.collect_identities_by_attr(self.root, attr_name, &mut out);
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn collect_identities_by_attr(
        &self,
        node: NodeId,
        attr_name: &str,
        out: &mut Vec<(String, OracleNodeId)>,
    ) {
        let n = self.node(node);
        if let NodeKind::Element { .. } = &n.kind {
            for attr in &n.attrs {
                if attr.name == attr_name && attr.namespace.is_none() {
                    out.push((attr.value.clone(), OracleNodeId(node)));
                }
            }
        }
        for &child in &n.children {
            self.collect_identities_by_attr(child, attr_name, out);
        }
    }

    /// Assert that this renderer's mock DOM matches the DOM described by an `rsx!` block.
    ///
    /// The expected side is built by walking the VNode tree of a throwaway `VirtualDom`
    /// directly (via `vdom_snapshot`), without going through any `WriteMutations` path.
    /// The actual side is this oracle's mock DOM, which was built by applying every
    /// mutation emitted by the renderer under test. Equality therefore validates that
    /// the mutation stream produced the correct DOM.
    pub fn assert_matches(&self, expected: fn() -> Element) {
        let mut tmp = VirtualDom::new(expected);
        tmp.rebuild_in_place();
        let expected_snapshot = vdom_snapshot(&tmp);
        pretty_assertions::assert_eq!(
            self.snapshot(),
            expected_snapshot,
            "renderer DOM diverged from expected rsx tree"
        );
    }

    fn alloc(&mut self, kind: NodeKind) -> NodeId {
        let id = self.arena.len();
        self.arena.push(Some(Node {
            kind,
            attrs: Vec::new(),
            listeners: Vec::new(),
            children: Vec::new(),
            parent: None,
        }));
        self.node_to_elements.push(Vec::new());
        id
    }

    fn node(&self, id: NodeId) -> &Node {
        self.arena
            .get(id)
            .and_then(Option::as_ref)
            .unwrap_or_else(|| panic!("renderer referenced dead node {id}"))
    }

    fn node_mut(&mut self, id: NodeId) -> &mut Node {
        self.arena
            .get_mut(id)
            .and_then(Option::as_mut)
            .unwrap_or_else(|| panic!("renderer referenced dead node {id}"))
    }

    fn set_element_mapping(&mut self, id: ElementId, node: NodeId) {
        if id.0 == usize::MAX {
            panic!("renderer cannot map ElementId(usize::MAX)");
        }
        if self.element_to_node.len() <= id.0 {
            self.element_to_node.resize(id.0 + 1, None);
        }
        if let Some(old) = self.element_to_node[id.0] {
            if old == node {
                return;
            }
            if old != node && self.arena.get(old).is_some_and(Option::is_some) {
                if self.node(old).parent.is_none() {
                    self.drop_subtree(old);
                } else {
                    panic!(
                        "renderer remapped live ElementId({}) from node {old} to node {node}",
                        id.0
                    );
                }
            }
        }
        self.clear_element_mapping(id);
        self.element_to_node[id.0] = Some(node);
        self.node_to_elements[node].push(id);
    }

    fn clear_element_mapping(&mut self, id: ElementId) {
        let Some(mapped) = self.element_to_node.get_mut(id.0).and_then(Option::take) else {
            return;
        };
        let Some(elements) = self.node_to_elements.get_mut(mapped) else {
            return;
        };
        if let Some(index) = elements.iter().position(|&element| element == id) {
            elements.swap_remove(index);
        }
    }

    fn lookup(&self, id: ElementId) -> NodeId {
        self.element_to_node
            .get(id.0)
            .and_then(|id| *id)
            .filter(|&node| self.arena.get(node).is_some_and(Option::is_some))
            .unwrap_or_else(|| panic!("renderer asked for unknown ElementId({})", id.0))
    }

    /// Recursively materialize a template node. Mirrors what `native-dom` and the JS
    /// interpreter do: `TemplateNode::Dynamic` becomes a real placeholder node, so
    /// mutation paths can be walked as plain positional child indices.
    fn clone_template(&mut self, template: &TemplateNode) -> NodeId {
        match template {
            TemplateNode::Element {
                tag,
                namespace,
                attrs,
                children,
            } => {
                let id = self.alloc(NodeKind::Element {
                    tag: (*tag).to_string(),
                    namespace: namespace.map(ToString::to_string),
                });
                for attr in *attrs {
                    if let TemplateAttribute::Static {
                        name,
                        value,
                        namespace,
                    } = attr
                    {
                        self.set_attr(
                            id,
                            (*name).to_string(),
                            namespace.map(ToString::to_string),
                            (*value).to_string(),
                        );
                    }
                }
                let child_ids: Vec<NodeId> = children
                    .iter()
                    .map(|child| {
                        let child_id = self.clone_template(child);
                        self.node_mut(child_id).parent = Some(id);
                        child_id
                    })
                    .collect();
                self.node_mut(id).children = child_ids;
                id
            }
            TemplateNode::Text { text } => self.alloc(NodeKind::Text((*text).to_string())),
            TemplateNode::Dynamic { .. } => self.alloc(NodeKind::Placeholder),
        }
    }

    /// Walk from `start` through `path`, treating each segment as a positional child
    /// index. Since `TemplateNode::Dynamic` slots are materialized as real placeholder
    /// nodes (see `clone_template`), positional indices line up with the paths that
    /// `dioxus_core` emits.
    fn walk_path(&self, start: NodeId, path: &[u8]) -> NodeId {
        let mut current = start;
        for &segment in path {
            let parent = self.node(current);
            current = *parent.children.get(segment as usize).unwrap_or_else(|| {
                panic!(
                    "renderer path {path:?} walked past node {current}; child index {segment} out of bounds (len {})",
                    parent.children.len()
                )
            });
        }
        current
    }

    fn pop_nodes(&mut self, m: usize) -> Vec<NodeId> {
        let available = self.stack.len().saturating_sub(1);
        if m > available {
            panic!(
                "renderer stack underflow: tried to pop {m} node(s), only {available} available"
            );
        }
        let split = self.stack.len() - m;
        let mut nodes = std::mem::take(&mut self.popped_nodes);
        nodes.clear();
        nodes.extend(self.stack.drain(split..));
        nodes
    }

    fn recycle_popped_nodes(&mut self, mut nodes: Vec<NodeId>) {
        nodes.clear();
        self.popped_nodes = nodes;
    }

    fn position_in_parent(&self, node: NodeId) -> (NodeId, usize) {
        let parent = self
            .node(node)
            .parent
            .unwrap_or_else(|| panic!("node {node} has no parent"));
        let index = self
            .node(parent)
            .children
            .iter()
            .position(|&child| child == node)
            .unwrap_or_else(|| panic!("node {node} is missing from parent {parent}"));
        (parent, index)
    }

    fn detach(&mut self, node: NodeId) -> (NodeId, usize) {
        let (parent, index) = self.position_in_parent(node);
        let removed = self.node_mut(parent).children.remove(index);
        debug_assert_eq!(removed, node);
        self.node_mut(node).parent = None;
        (parent, index)
    }

    fn unhook(&mut self, node: NodeId) {
        if self.node(node).parent.is_some() {
            self.detach(node);
        }
    }

    fn unhook_all(&mut self, nodes: &[NodeId]) {
        for &node in nodes {
            self.unhook(node);
        }
    }

    fn insert_detached(&mut self, parent: NodeId, index: usize, nodes: &mut Vec<NodeId>) {
        if index > self.node(parent).children.len() {
            panic!(
                "renderer insertion index {index} out of bounds for parent {parent} with {} children",
                self.node(parent).children.len()
            );
        }
        for &node in nodes.iter() {
            self.node_mut(node).parent = Some(parent);
        }
        let parent_node = self.node_mut(parent);
        for (offset, node) in nodes.drain(..).enumerate() {
            parent_node.children.insert(index + offset, node);
        }
    }

    fn append_detached(&mut self, parent: NodeId, nodes: &mut Vec<NodeId>) {
        for &node in nodes.iter() {
            self.node_mut(node).parent = Some(parent);
        }
        self.node_mut(parent).children.extend(nodes.drain(..));
    }

    fn drop_subtree(&mut self, node: NodeId) {
        if node == self.root {
            panic!("renderer cannot drop document root");
        }
        let node_data = self.arena[node]
            .take()
            .unwrap_or_else(|| panic!("renderer tried to drop already-dead node {node}"));
        for id in self.node_to_elements[node].drain(..) {
            if let Some(mapped) = self.element_to_node.get_mut(id.0) {
                if *mapped == Some(node) {
                    *mapped = None;
                }
            }
        }
        for child in node_data.children {
            // Children of a dropped subtree are still attached (in the dead node's
            // `children`), so just recurse — no need to detach them first.
            self.arena[child]
                .as_mut()
                .map(|n| n.parent = None)
                .unwrap_or(());
            self.drop_subtree(child);
        }
    }

    fn assert_element(&self, node: NodeId, operation: &str) {
        if !matches!(self.node(node).kind, NodeKind::Element { .. }) {
            panic!(
                "{operation} expected an element node, got {:?}",
                self.node(node).kind
            );
        }
    }

    fn set_attr(&mut self, node: NodeId, name: String, namespace: Option<String>, value: String) {
        self.assert_element(node, "set_attribute");
        let attrs = &mut self.node_mut(node).attrs;
        match attrs
            .binary_search_by(|attr| attr_key(attr).cmp(&(name.as_str(), namespace.as_deref())))
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

    fn remove_attr(&mut self, node: NodeId, name: &str, namespace: Option<&str>) {
        self.assert_element(node, "remove_attribute");
        let attrs = &mut self.node_mut(node).attrs;
        if let Ok(index) = attrs.binary_search_by(|attr| attr_key(attr).cmp(&(name, namespace))) {
            attrs.remove(index);
        }
    }

    fn snapshot_node_eq(&self, node: NodeId, other: &Self, other_node: NodeId) -> bool {
        let node_data = self.node(node);
        let other_node_data = other.node(other_node);
        match (&node_data.kind, &other_node_data.kind) {
            (NodeKind::Document, NodeKind::Document) => {
                self.visible_children_eq(node, other, other_node)
            }
            (
                NodeKind::Element { tag, namespace },
                NodeKind::Element {
                    tag: other_tag,
                    namespace: other_namespace,
                },
            ) => {
                tag == other_tag
                    && namespace == other_namespace
                    && node_data.attrs == other_node_data.attrs
                    && node_data.listeners == other_node_data.listeners
                    && self.visible_children_eq(node, other, other_node)
            }
            (NodeKind::Text(text), NodeKind::Text(other_text)) => text == other_text,
            (NodeKind::Placeholder, NodeKind::Placeholder) => true,
            _ => false,
        }
    }

    fn visible_children_eq(&self, node: NodeId, other: &Self, other_node: NodeId) -> bool {
        let mut children = self
            .node(node)
            .children
            .iter()
            .copied()
            .filter(|&child| !matches!(self.node(child).kind, NodeKind::Placeholder));
        let mut other_children = other
            .node(other_node)
            .children
            .iter()
            .copied()
            .filter(|&child| !matches!(other.node(child).kind, NodeKind::Placeholder));

        loop {
            match (children.next(), other_children.next()) {
                (Some(child), Some(other_child)) => {
                    if !self.snapshot_node_eq(child, other, other_child) {
                        return false;
                    }
                }
                (None, None) => return true,
                _ => return false,
            }
        }
    }

    fn snapshot_node(&self, node: NodeId) -> Option<SnapshotNode> {
        let node_data = self.node(node);
        match &node_data.kind {
            NodeKind::Document => panic!("document root is not part of snapshots"),
            NodeKind::Element { tag, namespace } => Some(SnapshotNode::Element {
                tag: tag.clone(),
                namespace: namespace.clone(),
                attrs: node_data.attrs.clone(),
                listeners: node_data.listeners.clone(),
                children: node_data
                    .children
                    .iter()
                    .filter_map(|&child| self.snapshot_node(child))
                    .collect(),
            }),
            NodeKind::Placeholder => None,
            NodeKind::Text(text) => Some(SnapshotNode::Text(text.clone())),
        }
    }
}

impl WriteMutations for RendererOracle {
    fn append_children(&mut self, id: ElementId, m: usize) {
        self.edit_counters.inserts += 1;
        let mut nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        self.append_detached(self.lookup(id), &mut nodes);
        self.recycle_popped_nodes(nodes);
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        let top = *self
            .stack
            .last()
            .expect("renderer stack unexpectedly empty during assign_node_id");
        let node = self.walk_path(top, path);
        self.set_element_mapping(id, node);
    }

    fn create_placeholder(&mut self, id: ElementId) {
        let node = self.alloc(NodeKind::Placeholder);
        self.set_element_mapping(id, node);
        self.stack.push(node);
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        self.edit_counters.create_texts += 1;
        let node = self.alloc(NodeKind::Text(value.to_string()));
        self.set_element_mapping(id, node);
        self.stack.push(node);
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        self.edit_counters.loads += 1;
        let root = template
            .roots()
            .get(index)
            .unwrap_or_else(|| panic!("renderer loaded missing template root {index}"));
        let node = self.clone_template(root);
        self.set_element_mapping(id, node);
        self.stack.push(node);
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        self.edit_counters.replaces += 1;
        let mut nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let target = self.lookup(id);
        let (parent, index) = self.detach(target);
        self.drop_subtree(target);
        self.insert_detached(parent, index, &mut nodes);
        self.recycle_popped_nodes(nodes);
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.edit_counters.inserts += 1;
        // Order matters: pop the stack first, then walk_path reads from the top.
        // Mirrors `native-dom`'s `replace_placeholder_with_nodes` (mutation_writer.rs).
        let mut nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let top = *self
            .stack
            .last()
            .expect("renderer stack unexpectedly empty during replace_placeholder_with_nodes");
        let anchor = self.walk_path(top, path);
        let (parent, index) = self.detach(anchor);
        self.drop_subtree(anchor);
        self.insert_detached(parent, index, &mut nodes);
        self.recycle_popped_nodes(nodes);
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        self.edit_counters.inserts += 1;
        let mut nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let anchor = self.lookup(id);
        let (parent, index) = self.position_in_parent(anchor);
        self.insert_detached(parent, index + 1, &mut nodes);
        self.recycle_popped_nodes(nodes);
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        self.edit_counters.inserts += 1;
        let mut nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let anchor = self.lookup(id);
        let (parent, index) = self.position_in_parent(anchor);
        self.insert_detached(parent, index, &mut nodes);
        self.recycle_popped_nodes(nodes);
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        self.edit_counters.set_attrs += 1;
        let node = self.lookup(id);
        match attr_to_string(value) {
            Some(value) => {
                self.set_attr(node, name.to_string(), ns.map(ToString::to_string), value)
            }
            None => self.remove_attr(node, name, ns),
        }
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        self.edit_counters.set_texts += 1;
        let node = self.lookup(id);
        match &mut self.node_mut(node).kind {
            NodeKind::Text(text) => *text = value.to_string(),
            other => panic!("set_node_text expected text node, got {other:?}"),
        }
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        let node = self.lookup(id);
        self.assert_element(node, "create_event_listener");
        let target = EventListenerTarget { name, id };
        if !self.historical_event_listener_targets.contains(&target) {
            self.historical_event_listener_targets.push(target);
        }
        let listeners = &mut self.node_mut(node).listeners;
        let name = name.to_string();
        match listeners.binary_search(&name) {
            Ok(_) => {}
            Err(index) => listeners.insert(index, name),
        }
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        let node = self.lookup(id);
        self.assert_element(node, "remove_event_listener");
        let listeners = &mut self.node_mut(node).listeners;
        let name = name.to_string();
        match listeners.binary_search(&name) {
            Ok(index) => {
                listeners.remove(index);
            }
            Err(_) => panic!("renderer removed missing event listener {name:?}"),
        }
    }

    fn remove_node(&mut self, id: ElementId) {
        self.edit_counters.removes += 1;
        if id.0 == 0 {
            panic!("renderer cannot remove document root ElementId(0)");
        }
        let node = self.lookup(id);
        self.detach(node);
        self.drop_subtree(node);
    }

    fn push_root(&mut self, id: ElementId) {
        self.edit_counters.pushes += 1;
        if id.0 == 0 {
            panic!("dioxus emitted PushRoot {{ id: ElementId(0) }}");
        }
        if id.0 == usize::MAX {
            panic!("dioxus emitted PushRoot {{ id: ElementId(usize::MAX) }}");
        }
        let node = self.lookup(id);
        self.stack.push(node);
    }
}

impl fmt::Debug for RendererOracle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RendererOracle")
            .field("snapshot", &self.snapshot())
            .field("pending_stack_nodes", &self.pending_stack_nodes())
            .finish()
    }
}

//! A fast in-memory renderer for validating Dioxus mutation streams.
//!
//! `RendererOracle` implements [`dioxus_core::WriteMutations`] and maintains a
//! compact mock DOM. It is intended for tests and fuzzers that need renderer
//! semantics without webviews, JS bindings, layout, or serialization.

use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, Element, ElementId, Mutations, ScopeId, Template,
    TemplateAttribute, TemplateNode, VNode, VirtualDom, WriteMutations, consume_context,
    generation,
};
use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// Backwards-compatible name for callers that want a plain mock renderer.
pub type MockRenderer = RendererOracle;

/// Backwards-compatible name for the renderer's stable structural snapshot.
pub type Canonical = SnapshotNode;

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
    /// For each child, its template index within this element's template. Statics get
    /// their position in the template; slot content shares the slot's template index;
    /// nodes appended without template context get `u8::MAX` (sentinel meaning "no
    /// template position, lives at the end").
    child_template_indices: Vec<u8>,
    parent: Option<NodeId>,
}

const NO_TEMPLATE_INDEX: u8 = u8::MAX;

/// A stable, comparable view of the mock renderer tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotNode {
    Element {
        tag: String,
        namespace: Option<String>,
        attrs: Vec<SnapshotAttr>,
        listeners: Vec<String>,
        children: Vec<SnapshotNode>,
    },
    Text(String),
}

fn format_snapshot_mismatch(
    message: &str,
    actual: &[SnapshotNode],
    expected: &[SnapshotNode],
) -> String {
    format!("{message}\n\nrenderer snapshot:\n{actual:#?}\n\nexpected snapshot:\n{expected:#?}")
}

/// A stable attribute snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotAttr {
    pub name: String,
    pub namespace: Option<String>,
    pub value: String,
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
#[derive(Default, Debug, Clone, PartialEq, Eq)]
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
    stack: Vec<NodeId>,
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
                child_template_indices: Vec::new(),
                parent: None,
            })],
            element_to_node: vec![Some(root)],
            stack: vec![root],
            root,
            edit_counters: EditSummary::default(),
            historical_event_listener_targets: Vec::new(),
        }
    }

    /// Return a category-level summary of the edits applied during the most
    /// recent `rebuild` / `render` / `wait_and_render` call. See [`EditSummary`].
    pub fn last_edit_summary(&self) -> EditSummary {
        self.edit_counters.clone()
    }

    /// Return every event listener target attached since the last clear/rebuild.
    pub fn historical_event_listener_targets(&self) -> &[EventListenerTarget] {
        &self.historical_event_listener_targets
    }

    /// Remove all nodes and reset the renderer to an empty document.
    pub fn clear(&mut self) {
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

    /// Return the number of non-document nodes currently left on the mutation stack.
    pub fn pending_stack_nodes(&self) -> usize {
        self.stack.len().saturating_sub(1)
    }

    /// Return true when no mutation-created nodes are left on the stack.
    pub fn is_stack_clean(&self) -> bool {
        self.stack == [self.root]
    }

    /// Assert that the mutation stack only contains the document root.
    pub fn assert_stack_clean(&self) {
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

    /// Assert that this renderer's snapshot matches an expected snapshot.
    pub fn assert_snapshot_eq(&self, expected: &[SnapshotNode]) {
        if let Err(error) = self.check_snapshot_eq(expected) {
            panic!("{error}");
        }
    }

    /// Check that this renderer's snapshot matches an expected snapshot.
    pub fn check_snapshot_eq(&self, expected: &[SnapshotNode]) -> Result<(), String> {
        let actual = self.snapshot();
        if actual == expected {
            Ok(())
        } else {
            Err(format_snapshot_mismatch(
                "renderer snapshot diverged from expected tree",
                &actual,
                expected,
            ))
        }
    }

    /// Assert that this renderer's snapshot matches a fresh rebuild of `app`.
    pub fn assert_matches_fresh(&self, app: fn() -> Element) {
        self.assert_snapshot_eq(&fresh_snapshot(app));
    }

    /// Assert that this renderer's snapshot matches the raw rendered VDOM tree.
    pub fn assert_matches_vdom(&self, vdom: &VirtualDom) {
        if let Err(error) = self.check_matches_vdom(vdom) {
            panic!("{error}");
        }
    }

    /// Check that this renderer's snapshot matches the raw rendered VDOM tree.
    pub fn check_matches_vdom(&self, vdom: &VirtualDom) -> Result<(), String> {
        let actual = self.snapshot();
        let expected = vdom_snapshot(vdom);
        if actual == expected {
            Ok(())
        } else {
            Err(format_snapshot_mismatch(
                "renderer snapshot diverged from raw VirtualDom tree",
                &actual,
                &expected,
            ))
        }
    }

    /// Rebuild `vdom` into this renderer and assert the renderer stack is clean.
    pub fn rebuild(&mut self, vdom: &mut VirtualDom) {
        self.clear();
        vdom.rebuild(self);
        self.assert_stack_clean();
    }

    /// Drain pending immediate work from `vdom` into this renderer and assert the stack is clean.
    pub fn render(&mut self, vdom: &mut VirtualDom) {
        self.edit_counters = EditSummary::default();
        vdom.render_immediate(self);
        self.assert_stack_clean();
    }

    /// Await pending work on `vdom`, then drain it into this renderer.
    pub async fn wait_and_render(&mut self, vdom: &mut VirtualDom) {
        vdom.wait_for_work().await;
        self.render(vdom);
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
            child_template_indices: Vec::new(),
            parent: None,
        }));
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
        self.element_to_node[id.0] = Some(node);
    }

    fn lookup(&self, id: ElementId) -> NodeId {
        self.element_to_node
            .get(id.0)
            .and_then(|id| *id)
            .filter(|&node| self.arena.get(node).is_some_and(Option::is_some))
            .unwrap_or_else(|| panic!("renderer asked for unknown ElementId({})", id.0))
    }

    /// Recursively materialize a template node. Returns the new node id for static
    /// elements/text, or `None` for `TemplateNode::Dynamic` since dynamic slots have
    /// no DOM presence until content is inserted into them.
    fn clone_template(&mut self, template: &TemplateNode) -> Option<NodeId> {
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
                let mut child_ids = Vec::new();
                let mut child_tis = Vec::new();
                for (template_idx, child) in children.iter().enumerate() {
                    if let Some(child_id) = self.clone_template(child) {
                        self.node_mut(child_id).parent = Some(id);
                        child_ids.push(child_id);
                        child_tis.push(template_idx as u8);
                    }
                }
                let node = self.node_mut(id);
                node.children = child_ids;
                node.child_template_indices = child_tis;
                Some(id)
            }
            TemplateNode::Text { text } => Some(self.alloc(NodeKind::Text((*text).to_string()))),
            TemplateNode::Dynamic { .. } => None,
        }
    }

    /// Walk from `start` through `path`, treating each segment as a template index.
    /// Returns the node id of the static child at each step. Panics if any step
    /// fails to resolve — paths must only end at slot positions (handled by
    /// [`Self::walk_slot_path`]).
    fn walk_path(&self, start: NodeId, path: &[u8]) -> NodeId {
        let mut current = start;
        for &segment in path {
            current = self
                .find_child_with_template_index(current, segment)
                .unwrap_or_else(|| {
                    panic!(
                        "renderer path {path:?} walked past node {current}; missing child template-index {segment}"
                    )
                });
        }
        current
    }

    fn find_child_with_template_index(&self, parent: NodeId, ti: u8) -> Option<NodeId> {
        let parent_node = self.node(parent);
        for (idx, &this_ti) in parent_node.child_template_indices.iter().enumerate() {
            if this_ti == ti {
                return Some(parent_node.children[idx]);
            }
        }
        None
    }

    /// Resolve `path` ending at a slot position. Returns `(parent_node, slot_ti)`
    /// where `parent_node` is the element containing the slot and `slot_ti` is the
    /// template index of the slot within that parent. The caller is responsible
    /// for finding the right DOM insertion position from these.
    fn walk_to_slot_parent(&self, start: NodeId, path: &[u8]) -> (NodeId, u8) {
        let (&leaf, intermediate) = path
            .split_last()
            .expect("renderer was asked to walk an empty slot path");
        let parent = self.walk_path(start, intermediate);
        (parent, leaf)
    }

    fn pop_nodes(&mut self, m: usize) -> Vec<NodeId> {
        let available = self.stack.len().saturating_sub(1);
        if m > available {
            panic!(
                "renderer stack underflow: tried to pop {m} node(s), only {available} available"
            );
        }
        let split = self.stack.len() - m;
        self.stack.split_off(split)
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

    fn detach(&mut self, node: NodeId) -> (NodeId, usize, u8) {
        let (parent, index) = self.position_in_parent(node);
        let parent_node = self.node_mut(parent);
        let removed = parent_node.children.remove(index);
        let ti = parent_node.child_template_indices.remove(index);
        debug_assert_eq!(removed, node);
        self.node_mut(node).parent = None;
        (parent, index, ti)
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

    fn insert_detached(&mut self, parent: NodeId, index: usize, nodes: Vec<NodeId>, ti: u8) {
        if index > self.node(parent).children.len() {
            panic!(
                "renderer insertion index {index} out of bounds for parent {parent} with {} children",
                self.node(parent).children.len()
            );
        }
        for &node in &nodes {
            self.node_mut(node).parent = Some(parent);
        }
        let parent_node = self.node_mut(parent);
        for (offset, node) in nodes.into_iter().enumerate() {
            parent_node.children.insert(index + offset, node);
            parent_node
                .child_template_indices
                .insert(index + offset, ti);
        }
    }

    fn append_detached(&mut self, parent: NodeId, nodes: Vec<NodeId>, ti: u8) {
        for &node in &nodes {
            self.node_mut(node).parent = Some(parent);
        }
        let parent_node = self.node_mut(parent);
        let added = nodes.len();
        parent_node.children.extend(nodes);
        parent_node
            .child_template_indices
            .extend(std::iter::repeat(ti).take(added));
    }

    /// Find the insertion index in `parent` for content belonging to the slot at
    /// template index `slot_ti`. Slot content is grouped together: this returns the
    /// position right after the last existing child whose template index is `<=
    /// slot_ti`. Children with `NO_TEMPLATE_INDEX` (append-only content) live at the
    /// end regardless of `slot_ti`.
    fn slot_insert_position(&self, parent: NodeId, slot_ti: u8) -> usize {
        let parent_node = self.node(parent);
        let mut pos = 0;
        for (i, &ti) in parent_node.child_template_indices.iter().enumerate() {
            if ti == NO_TEMPLATE_INDEX {
                continue;
            }
            if ti <= slot_ti {
                pos = i + 1;
            } else {
                return pos;
            }
        }
        // Either ran out of template-indexed children (insert at `pos`) or only
        // append-only children remain past `pos` — insert at `pos` to stay before
        // the append-only tail.
        pos
    }

    fn drop_subtree(&mut self, node: NodeId) {
        if node == self.root {
            panic!("renderer cannot drop document root");
        }
        let node_data = self.arena[node]
            .take()
            .unwrap_or_else(|| panic!("renderer tried to drop already-dead node {node}"));
        for mapped in &mut self.element_to_node {
            if *mapped == Some(node) {
                *mapped = None;
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
        let nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        self.append_detached(self.lookup(id), nodes, NO_TEMPLATE_INDEX);
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
        let node = self
            .clone_template(root)
            .unwrap_or_else(|| panic!("renderer cannot load a Dynamic root template"));
        self.set_element_mapping(id, node);
        self.stack.push(node);
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        self.edit_counters.replaces += 1;
        let nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let target = self.lookup(id);
        let (parent, index, ti) = self.detach(target);
        self.drop_subtree(target);
        self.insert_detached(parent, index, nodes, ti);
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.edit_counters.inserts += 1;
        let nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let top = *self
            .stack
            .last()
            .expect("renderer stack unexpectedly empty during replace_placeholder_with_nodes");
        let (parent, slot_ti) = self.walk_to_slot_parent(top, path);
        let insert_index = self.slot_insert_position(parent, slot_ti);
        self.insert_detached(parent, insert_index, nodes, slot_ti);
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        self.edit_counters.inserts += 1;
        let nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let anchor = self.lookup(id);
        let (parent, index) = self.position_in_parent(anchor);
        let ti = self.node(parent).child_template_indices[index];
        self.insert_detached(parent, index + 1, nodes, ti);
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        self.edit_counters.inserts += 1;
        let nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let anchor = self.lookup(id);
        let (parent, index) = self.position_in_parent(anchor);
        let ti = self.node(parent).child_template_indices[index];
        self.insert_detached(parent, index, nodes, ti);
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

/// The steps for a [`Sequence`], handed to the source app via a root context so
/// the dispatcher can pick the current state by `generation()`.
#[derive(Clone)]
struct SequenceSteps(Rc<Vec<StepSource>>);

/// The step a [`Sequence`]'s expected-side `VirtualDom` should render, passed in
/// via a root context so the same dispatch function works for both source and
/// expected sides.
#[derive(Clone)]
struct ExpectedStep(Rc<StepSource>);

/// Drive a `VirtualDom` through an ordered sequence of states. Each step is an
/// `rsx!` block that plays both roles: the content the source component renders
/// for that generation and the expected DOM the oracle asserts after rendering.
///
/// Usage:
///
/// ```ignore
/// Sequence::new()
///     .step(rsx! { div { "a" } })
///     .step(rsx! { div { "b" } })
///     .run();
/// ```
///
/// For parameterized steps, call a helper that returns `Element`:
///
/// ```ignore
/// fn divs(keys: &[i32]) -> Element { rsx! { for k in keys.iter().copied() { div { "{k}" } } } }
/// Sequence::new()
///     .step(divs(&[1, 2, 3]))
///     .step(divs(&[3, 2, 1]))
///     .run();
/// ```
///
/// The source app dispatches on `dioxus_core::generation()` to pick the current
/// step (cloned from a root context — no globals, no unsafe). Between steps
/// `Sequence` marks `ScopeId::APP` dirty and renders. The expected DOM is built
/// by walking the VNode tree of the same step in a throwaway `VirtualDom` —
/// independent of the renderer's mutation path.
/// How a step's source/expected content is produced.
///
/// `Static` is a pre-built `Element` — what `rsx!{...}` evaluates to outside any
/// runtime. Works for handler-free, signal-free content.
///
/// `Lazy` is a closure invoked inside the Dioxus runtime each time the step
/// renders. Required for rsx that creates event handlers, reads signals, or
/// otherwise needs runtime context to construct.
enum StepSource {
    Static(Element),
    Lazy(Box<dyn Fn() -> Element>),
}

impl StepSource {
    fn produce(&self) -> Element {
        match self {
            StepSource::Static(e) => e.clone(),
            StepSource::Lazy(f) => f(),
        }
    }
}

/// One entry in a [`Sequence`]'s timeline. Steps and interludes interleave in
/// authoring order — there's no parallel-indexed second list.
enum SequenceItem {
    /// An expected DOM state. Doubles as the source content for that generation.
    Step(StepSource),
    /// A side-effect that runs in authoring position. Useful for firing synthetic
    /// events, reading context, or making side-channel assertions on the
    /// `VirtualDom` between renders. Receives the live oracle so that event
    /// targets can be resolved semantically (`oracle.element_id_by_tag(...)`,
    /// `oracle.element_id_by_attr(...)`) instead of by raw `ElementId(N)`
    /// literal.
    Interlude(Box<dyn FnMut(&mut VirtualDom, &RendererOracle)>),
}

/// An assertion registered against the [`EditSummary`] captured at a specific
/// step. `step` is the 0-indexed transition (step 0 = initial rebuild, step 1 =
/// first rerender, ...). The closure runs after the step's render completes and
/// is free to panic to signal failure.
struct EditSummaryAssertion {
    step: usize,
    check: Box<dyn Fn(&EditSummary)>,
}

#[must_use]
pub struct Sequence {
    items: Vec<SequenceItem>,
    identity_attr: Option<String>,
    edit_summary_assertions: Vec<EditSummaryAssertion>,
}

fn sequence_dispatch() -> Element {
    let steps = consume_context::<SequenceSteps>();
    let idx = generation().min(steps.0.len() - 1);
    steps.0[idx].produce()
}

fn expected_dispatch() -> Element {
    let step = consume_context::<ExpectedStep>();
    step.0.produce()
}

impl Sequence {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            identity_attr: None,
            edit_summary_assertions: Vec::new(),
        }
    }

    /// Append a state from a pre-built `rsx!` block. The same `Element` is cloned
    /// for the source-side render and for the expected-DOM comparison. Use this
    /// for handler-free, signal-free content.
    pub fn step(mut self, state: Element) -> Self {
        self.items
            .push(SequenceItem::Step(StepSource::Static(state)));
        self
    }

    /// Append a state from a closure that runs *inside* the Dioxus runtime each
    /// time the step renders. Use this when the rsx contains event handlers or
    /// reads signals — those constructions require an active runtime.
    pub fn step_with(mut self, state: impl Fn() -> Element + 'static) -> Self {
        self.items
            .push(SequenceItem::Step(StepSource::Lazy(Box::new(state))));
        self
    }

    /// Append a side-effect that runs in authoring position — between the
    /// previous step's assertion and the next step's `mark_dirty`. The closure
    /// receives both the `VirtualDom` and the oracle's current view of the DOM
    /// so that event targets can be resolved semantically:
    ///
    /// ```ignore
    /// Sequence::new()
    ///     .step(rsx! { button { onclick: ..., "click me" } })
    ///     .interlude(|dom, oracle| {
    ///         let btn = oracle.element_id_by_tag("button");
    ///         dom.runtime().handle_event("click", event, btn);
    ///     })
    ///     .step(rsx! { button { onclick: ..., "clicked once" } })
    ///     .run();
    /// ```
    pub fn interlude(
        mut self,
        action: impl FnMut(&mut VirtualDom, &RendererOracle) + 'static,
    ) -> Self {
        self.items.push(SequenceItem::Interlude(Box::new(action)));
        self
    }

    /// Track per-node DOM identity across renders by the value of an HTML
    /// attribute on each element. After each step, the oracle records the
    /// `attr_value -> OracleNodeId` mapping; values that appear in two
    /// consecutive steps must map to the *same* `OracleNodeId`, otherwise the
    /// renderer dropped-and-recreated a node that should have been moved.
    ///
    /// Use this on tests that need to assert keyed-diffing identity (animation,
    /// focus, scroll position preservation):
    ///
    /// ```ignore
    /// Sequence::new()
    ///     .track_identity_by("id")
    ///     .step(|| rsx! { div { id: "0", "first" } div { id: "1", "second" } })
    ///     .step(|| rsx! { div { id: "1", "second" } div { id: "0", "first" } })
    ///     .run();
    /// ```
    pub fn track_identity_by(mut self, attr: &str) -> Self {
        self.identity_attr = Some(attr.to_string());
        self
    }

    /// Register an assertion against the [`EditSummary`] captured for the render
    /// at `step` (0-indexed: step 0 is the initial rebuild, step 1 is the first
    /// rerender, ...). Use this to guard structural diff properties that
    /// final-DOM snapshots cannot see — minimal move counts, in-place patches,
    /// no-op rerenders:
    ///
    /// ```ignore
    /// Sequence::new()
    ///     .step(rsx! { for k in [0,1,2] { div { key: "{k}", id: "{k}" } } })
    ///     .step(rsx! { for k in [2,0,1] { div { key: "{k}", id: "{k}" } } })
    ///     .assert_edit_summary(1, |s| {
    ///         assert!(s.pushes <= 1, "expected one move, got {} pushes", s.pushes);
    ///         assert_eq!(s.creates(), 0);
    ///     })
    ///     .run();
    /// ```
    ///
    /// Multiple assertions for the same step are allowed and all run.
    pub fn assert_edit_summary(
        mut self,
        step: usize,
        check: impl Fn(&EditSummary) + 'static,
    ) -> Self {
        self.edit_summary_assertions.push(EditSummaryAssertion {
            step,
            check: Box::new(check),
        });
        self
    }

    /// Execute every item in order. Each `Step` renders the source and asserts
    /// the DOM matches; each `Interlude` runs its side-effect at that point in
    /// the timeline.
    pub fn run(mut self) {
        // Pull the steps into a shared list. Interludes don't reach the source
        // VDom — they manipulate it externally between renders.
        let just_steps: Vec<Rc<StepSource>> = self
            .items
            .iter_mut()
            .filter_map(|item| match item {
                SequenceItem::Step(src) => {
                    // Replace the StepSource with a placeholder so we can move it
                    // out (Element is Clone but Box<dyn Fn> isn't); we'll share
                    // each step via Rc to allow both source and expected sides.
                    let taken = std::mem::replace(src, StepSource::Static(VNode::empty()));
                    Some(Rc::new(taken))
                }
                SequenceItem::Interlude(_) => None,
            })
            .collect();
        assert!(!just_steps.is_empty(), "Sequence needs at least one step");

        let source_steps: Vec<StepSource> = just_steps
            .iter()
            .map(|s| match s.as_ref() {
                StepSource::Static(e) => StepSource::Static(e.clone()),
                // For Lazy we share via Rc through ExpectedStep; the source side
                // gets its own clone of the Rc-wrapped closure too.
                StepSource::Lazy(_) => StepSource::Lazy(Box::new({
                    let shared = s.clone();
                    move || shared.produce()
                })),
            })
            .collect();
        let steps_ctx = SequenceSteps(Rc::new(source_steps));
        let mut dom = VirtualDom::new(sequence_dispatch).with_root_context(steps_ctx);
        let mut oracle = RendererOracle::new();
        let identity_attr = self.identity_attr.clone();
        let mut prev_identities: Option<Vec<(String, OracleNodeId)>> = None;
        let mut step_index = 0usize;
        let max_step = just_steps.len();
        for assertion in &self.edit_summary_assertions {
            assert!(
                assertion.step < max_step,
                "assert_edit_summary references step {} but the sequence only has {} step(s)",
                assertion.step,
                max_step,
            );
        }

        for item in &mut self.items {
            match item {
                SequenceItem::Step(_) => {
                    if step_index == 0 {
                        oracle.rebuild(&mut dom);
                    } else {
                        dom.mark_dirty(ScopeId::APP);
                        oracle.render(&mut dom);
                    }
                    assert_step(&oracle, &just_steps[step_index]);
                    if let Some(attr) = identity_attr.as_deref() {
                        let current = oracle.identities_by_attr(attr);
                        if let Some(prev) = prev_identities.as_deref() {
                            assert_identity_preserved(prev, &current, attr, step_index);
                        }
                        prev_identities = Some(current);
                    }
                    let summary = oracle.last_edit_summary();
                    for assertion in &self.edit_summary_assertions {
                        if assertion.step == step_index {
                            (assertion.check)(&summary);
                        }
                    }
                    step_index += 1;
                }
                SequenceItem::Interlude(action) => {
                    action(&mut dom, &oracle);
                }
            }
        }
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

/// For each value that appears in both `prev` and `current`, assert that the
/// `OracleNodeId` is preserved. New values (added this step) and dropped values
/// (removed this step) are allowed; only common-value mismatches are a failure.
fn assert_identity_preserved(
    prev: &[(String, OracleNodeId)],
    current: &[(String, OracleNodeId)],
    attr: &str,
    step: usize,
) {
    use std::collections::HashMap;
    let prev_map: HashMap<&str, OracleNodeId> =
        prev.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    for (value, current_id) in current {
        if let Some(prev_id) = prev_map.get(value.as_str()) {
            assert_eq!(
                *prev_id, *current_id,
                "step {step}: node identity for `{attr}={value}` was not preserved \
                 (previous OracleNodeId {prev_id:?}, current {current_id:?}). \
                 This means the renderer dropped and recreated the node when it should \
                 have moved it — any browser-side state (animations, focus, scroll) \
                 would be lost.",
            );
        }
    }
}

/// Compare the oracle's current DOM against the DOM produced by rendering `step`
/// directly. Builds a throwaway `VirtualDom` whose component invokes the step
/// (via root-context dispatch) so handler/signal-bearing rsx is constructed
/// inside the runtime.
fn assert_step(oracle: &RendererOracle, step: &Rc<StepSource>) {
    let mut tmp = VirtualDom::new(expected_dispatch).with_root_context(ExpectedStep(step.clone()));
    tmp.rebuild_in_place();
    let expected_snapshot = vdom_snapshot(&tmp);
    pretty_assertions::assert_eq!(
        oracle.snapshot(),
        expected_snapshot,
        "renderer DOM diverged from expected rsx tree"
    );
}

/// Render `app` from scratch into a stable snapshot.
pub fn fresh_snapshot(app: fn() -> Element) -> Vec<SnapshotNode> {
    let mut vdom = VirtualDom::new(app);
    let mut renderer = RendererOracle::new();
    vdom.rebuild(&mut renderer);
    renderer.assert_stack_clean();
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
    vdom.render_immediate(renderer);
    renderer.assert_stack_clean();
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
                attrs: element_attrs,
                listeners,
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

fn attr_key(attr: &SnapshotAttr) -> (&str, Option<&str>) {
    (attr.name.as_str(), attr.namespace.as_deref())
}

fn attr_to_string(value: &AttributeValue) -> Option<String> {
    match value {
        AttributeValue::Text(s) => Some(s.clone()),
        AttributeValue::Bool(b) => Some(b.to_string()),
        AttributeValue::Float(f) => Some(f.to_string()),
        AttributeValue::Int(i) => Some(i.to_string()),
        AttributeValue::None => None,
        _ => Some("<opaque>".to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;

    fn simple_app() -> Element {
        rsx! {
            main { class: "root", "hello" }
        }
    }

    fn listener_app() -> Element {
        rsx! {
            button { onclick: move |_| {}, "go" }
        }
    }

    fn empty_dynamic_slot_app() -> Element {
        let show = false;
        rsx! {
            main {
                if show {
                    span { "hidden" }
                }
            }
        }
    }

    #[test]
    fn rebuilds_static_tree() {
        let snapshot = fresh_snapshot(simple_app);
        assert_eq!(
            snapshot,
            vec![SnapshotNode::Element {
                tag: "main".to_string(),
                namespace: None,
                attrs: vec![SnapshotAttr {
                    name: "class".to_string(),
                    namespace: None,
                    value: "root".to_string(),
                }],
                listeners: Vec::new(),
                children: vec![SnapshotNode::Text("hello".to_string())],
            }]
        );
    }

    #[test]
    fn tracks_event_listeners() {
        let snapshot = fresh_snapshot(listener_app);
        match &snapshot[..] {
            [SnapshotNode::Element { listeners, .. }] => assert_eq!(listeners, &["click"]),
            other => panic!("unexpected snapshot: {other:#?}"),
        }
    }

    #[test]
    fn records_historical_event_listener_targets() {
        let seen_id = std::rc::Rc::new(std::cell::Cell::new(None));
        Sequence::new()
            .step_with(|| {
                rsx! {
                    button { onclick: move |_| {}, "go" }
                }
            })
            .interlude({
                let seen_id = seen_id.clone();
                move |_, oracle| {
                    let id = oracle.element_id_by_tag("button");
                    seen_id.set(Some(id));
                    assert_eq!(
                        oracle.historical_event_listener_targets(),
                        &[EventListenerTarget { name: "click", id }]
                    );
                }
            })
            .step(rsx! {
                button { "go" }
            })
            .interlude({
                let seen_id = seen_id.clone();
                move |_, oracle| {
                    let id = seen_id.get().expect("listener id should be captured");
                    assert_eq!(
                        oracle.historical_event_listener_targets(),
                        &[EventListenerTarget { name: "click", id }]
                    );
                }
            })
            .run();
    }

    #[test]
    fn keeps_historical_event_listener_targets_after_node_removal() {
        let seen_id = std::rc::Rc::new(std::cell::Cell::new(None));
        Sequence::new()
            .step_with(|| {
                rsx! {
                    button { onclick: move |_| {}, "go" }
                }
            })
            .interlude({
                let seen_id = seen_id.clone();
                move |_, oracle| {
                    seen_id.set(Some(oracle.element_id_by_tag("button")));
                }
            })
            .step(rsx! {
                div { "gone" }
            })
            .interlude({
                let seen_id = seen_id.clone();
                move |_, oracle| {
                    let id = seen_id.get().expect("listener id should be captured");
                    assert_eq!(
                        oracle.historical_event_listener_targets(),
                        &[EventListenerTarget { name: "click", id }]
                    );
                }
            })
            .run();
    }

    #[test]
    fn empty_dynamic_slots_are_not_snapshot_nodes() {
        let snapshot = fresh_snapshot(empty_dynamic_slot_app);
        assert_eq!(
            snapshot,
            vec![SnapshotNode::Element {
                tag: "main".to_string(),
                namespace: None,
                attrs: Vec::new(),
                listeners: Vec::new(),
                children: Vec::new(),
            }]
        );
    }

    #[test]
    fn asserts_no_mutations_for_idle_vdom() {
        let mut vdom = VirtualDom::new(simple_app);
        let mut renderer = RendererOracle::new();
        vdom.rebuild(&mut renderer);
        renderer.assert_stack_clean();
        assert_no_mutations(&mut vdom);
    }

    #[test]
    fn assert_matches_happy_path() {
        let mut vdom = VirtualDom::new(simple_app);
        let mut renderer = RendererOracle::new();
        renderer.rebuild(&mut vdom);
        renderer.assert_matches(simple_app);
    }

    #[test]
    fn assert_matches_round_trips_listeners() {
        let mut vdom = VirtualDom::new(listener_app);
        let mut renderer = RendererOracle::new();
        renderer.rebuild(&mut vdom);
        renderer.assert_matches(listener_app);
    }

    #[test]
    fn sequence_walks_states_in_order() {
        Sequence::new()
            .step(rsx! { div { "a" } })
            .step(rsx! { div { "b" } })
            .step(rsx! { div { "c" } })
            .run();
    }

    #[test]
    fn sequence_tracks_identity_for_moved_nodes() {
        fn divs(keys: &[i32]) -> Element {
            rsx! {
                for k in keys.iter().copied() {
                    div { key: "{k}", id: "{k}", "{k}" }
                }
            }
        }
        // Reordering keyed nodes should *move* DOM nodes — identities preserved.
        Sequence::new()
            .track_identity_by("id")
            .step(divs(&[0, 1, 2, 3]))
            .step(divs(&[3, 0, 1, 2]))
            .step(divs(&[2, 3, 0, 1]))
            .run();
    }

    #[test]
    fn sequence_runs_interlude_between_steps() {
        use std::cell::Cell;
        thread_local! {
            static CALLS: Cell<usize> = const { Cell::new(0) };
        }
        CALLS.with(|c| c.set(0));
        Sequence::new()
            .step(rsx! { div { "a" } })
            .interlude(|_dom, _oracle| {
                CALLS.with(|c| c.set(c.get() + 1));
            })
            .step(rsx! { div { "b" } })
            .interlude(|_dom, _oracle| {
                CALLS.with(|c| c.set(c.get() + 1));
            })
            .step(rsx! { div { "c" } })
            .run();
        assert_eq!(CALLS.with(|c| c.get()), 2);
    }

    #[test]
    #[should_panic(expected = "node identity for `id=hot` was not preserved")]
    fn sequence_identity_check_catches_recreation() {
        // Two unkeyed elements of different tag — the diff has to drop the old
        // node and create a new one. The identity tracker catches that.
        Sequence::new()
            .track_identity_by("id")
            .step(rsx! { div { id: "hot", "before" } })
            .step(rsx! { span { id: "hot", "after" } })
            .run();
    }

    #[test]
    fn edit_summary_counts_rebuild_then_in_place_patch() {
        // First step builds the tree; rerender with the same shape but a
        // different *dynamic* text body should patch in place — same template,
        // just a new value for the dynamic slot.
        fn body(value: &str) -> Element {
            rsx! { div { id: "0", "{value}" } }
        }
        Sequence::new()
            .step(body("alpha"))
            .step(body("beta"))
            .assert_edit_summary(0, |s| {
                assert!(s.loads >= 1, "rebuild should load at least one template");
            })
            .assert_edit_summary(1, |s| {
                assert_eq!(s.loads, 0, "in-place text patch should not load templates");
                assert_eq!(s.set_texts, 1, "exactly one text patch expected");
                assert_eq!(s.removes, 0);
                assert_eq!(s.replaces, 0);
            })
            .run();
    }

    #[test]
    #[should_panic(expected = "expected one move")]
    fn edit_summary_assertion_fires_on_failure() {
        // Force the assertion to fail to confirm panics propagate.
        Sequence::new()
            .step(rsx! { div { id: "0" } })
            .step(rsx! { div { id: "0", "x" } })
            .assert_edit_summary(1, |_| panic!("expected one move"))
            .run();
    }

    #[test]
    #[should_panic(expected = "references step 5 but the sequence only has 2 step")]
    fn edit_summary_assertion_step_out_of_range() {
        Sequence::new()
            .step(rsx! { div {} })
            .step(rsx! { div {} })
            .assert_edit_summary(5, |_| {})
            .run();
    }

    #[test]
    #[should_panic(expected = "renderer DOM diverged from expected rsx tree")]
    fn assert_matches_fails_on_divergence() {
        fn other() -> Element {
            rsx! { main { class: "different", "hello" } }
        }
        let mut vdom = VirtualDom::new(simple_app);
        let mut renderer = RendererOracle::new();
        renderer.rebuild(&mut vdom);
        renderer.assert_matches(other);
    }
}

use crate::snapshot::{
    SnapshotAttr, SnapshotNode, attr_key, attr_to_string, format_snapshot_mismatch,
};
use crate::vdom_snapshot::{fresh_snapshot, vdom_snapshot};
use dioxus_core::{AttributeValue, Element, ElementId, VirtualDom, WriteMutations};
use std::fmt;

type NodeId = usize;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum NodeRole {
    Live,
    PrototypeRoot,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum StackSource {
    Live,
    PrototypeBuild,
    NewText,
    PrototypeClone,
}

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
    Text(String),
}

#[derive(Clone, Debug)]
struct Node {
    kind: NodeKind,
    attrs: Vec<SnapshotAttr>,
    listeners: Vec<String>,
    children: Vec<NodeId>,
    /// For each child, its logical static child index. Nodes appended without
    /// prototype context get `u8::MAX` (sentinel meaning "no static position,
    /// lives at the end").
    child_logical_indices: Vec<u8>,
    parent: Option<NodeId>,
}

const NO_LOGICAL_INDEX: u8 = u8::MAX;

/// A category-level summary of edits applied to the renderer in one render pass.
///
/// Counts edits by *kind* (prototype clone, create text, move, set attribute, ...)
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
    /// Prototype clone operations — a fresh element subtree was cloned from
    /// the renderer stack.
    pub loads: usize,
    /// `create_text` calls.
    pub create_texts: usize,
    /// `remove` calls.
    pub removes: usize,
    /// `replace_with` calls.
    pub replaces: usize,
    /// `insert_*` / `append_children` calls — placing nodes into the tree.
    pub inserts: usize,
    /// `push_id` calls — proxy for "an existing live node was brought onto the
    /// stack to be moved." A keyed reorder that moves N survivors emits N pushes.
    pub pushes: usize,
    /// `set_attribute` calls.
    pub set_attrs: usize,
    /// `set_text` calls — in-place text patches.
    pub set_texts: usize,
}

impl EditSummary {
    /// Total node-creation operations (`loads + create_texts`).
    pub fn creates(&self) -> usize {
        self.loads + self.create_texts
    }
}

/// An event listener target that has been attached during this renderer's lifetime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventListenerTarget {
    pub name: String,
    pub id: ElementId,
}

/// A fast mock renderer that applies Dioxus mutations into an in-memory tree.
pub struct RendererOracle {
    arena: Vec<Option<Node>>,
    node_roles: Vec<NodeRole>,
    element_to_node: Vec<Option<NodeId>>,
    stack: Vec<NodeId>,
    stack_sources: Vec<StackSource>,
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
    /// Create an empty document with `ElementId::from_raw(0)` mapped to the document root.
    pub fn new() -> Self {
        let root = 0;
        Self {
            arena: vec![Some(Node {
                kind: NodeKind::Document,
                attrs: Vec::new(),
                listeners: Vec::new(),
                children: Vec::new(),
                child_logical_indices: Vec::new(),
                parent: None,
            })],
            node_roles: vec![NodeRole::Live],
            element_to_node: vec![Some(root)],
            stack: vec![root],
            stack_sources: vec![StackSource::Live],
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

    /// Return the live [`ElementId`] mapped to the current stack node.
    pub fn current_stack_element_id(&self) -> Option<ElementId> {
        self.stack
            .last()
            .and_then(|&node| self.element_id_for_node(node))
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

    /// Compare this renderer's snapshot to an expected snapshot, returning whether they match.
    pub fn snapshot_eq(&self, expected: &[SnapshotNode]) -> bool {
        self.snapshot() == expected
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
    pub fn rebuild(&mut self, vdom: &mut VirtualDom) -> EditSummary {
        self.clear();
        vdom.rebuild(self);
        self.assert_stack_clean();
        self.edit_counters.clone()
    }

    /// Drain pending immediate work from `vdom` into this renderer and assert the stack is clean.
    pub fn render(&mut self, vdom: &mut VirtualDom) -> EditSummary {
        self.edit_counters = EditSummary::default();
        vdom.render_immediate(self);
        self.assert_stack_clean();
        self.edit_counters.clone()
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
    /// specific `ElementId::from_raw(N)` literal in test code: look up the target
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
                return Some(ElementId::from_raw(idx));
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
            child_logical_indices: Vec::new(),
            parent: None,
        }));
        self.node_roles.push(NodeRole::Live);
        id
    }

    fn push_stack(&mut self, node: NodeId, source: StackSource) {
        self.stack.push(node);
        self.stack_sources.push(source);
    }

    fn pop_stack(&mut self, operation: &str) -> (NodeId, StackSource) {
        let node = self
            .stack
            .pop()
            .unwrap_or_else(|| panic!("renderer stack unexpectedly empty during {operation}"));
        let source = self.stack_sources.pop().unwrap_or_else(|| {
            panic!("renderer stack source unexpectedly empty during {operation}")
        });
        (node, source)
    }

    fn top_stack(&self, operation: &str) -> (NodeId, StackSource) {
        let node = *self
            .stack
            .last()
            .unwrap_or_else(|| panic!("renderer stack unexpectedly empty during {operation}"));
        let source = *self.stack_sources.last().unwrap_or_else(|| {
            panic!("renderer stack source unexpectedly empty during {operation}")
        });
        (node, source)
    }

    fn replace_stack_top(&mut self, node: NodeId, source: StackSource, operation: &str) {
        *self
            .stack
            .last_mut()
            .unwrap_or_else(|| panic!("renderer stack unexpectedly empty during {operation}")) =
            node;
        *self.stack_sources.last_mut().unwrap_or_else(|| {
            panic!("renderer stack source unexpectedly empty during {operation}")
        }) = source;
    }

    fn stack_source_for_node(&self, node: NodeId) -> StackSource {
        match self.node_roles.get(node).copied().unwrap_or(NodeRole::Live) {
            NodeRole::Live => StackSource::Live,
            NodeRole::PrototypeRoot => StackSource::PrototypeBuild,
        }
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
        if id.raw() == usize::MAX {
            panic!("renderer cannot map ElementId::from_raw(usize::MAX)");
        }
        if self.element_to_node.len() <= id.raw() {
            self.element_to_node.resize(id.raw() + 1, None);
        }
        if let Some(old) = self.element_to_node[id.raw()] {
            if old != node && self.arena.get(old).is_some_and(Option::is_some) {
                if self.node(old).parent.is_none() {
                    self.drop_subtree(old);
                } else {
                    panic!(
                        "renderer remapped live ElementId::from_raw({}) from node {old} to node {node}",
                        id.raw()
                    );
                }
            }
        }
        self.element_to_node[id.raw()] = Some(node);
    }

    fn clear_element_mapping_for_node(&mut self, node: NodeId) {
        for mapped in &mut self.element_to_node {
            if *mapped == Some(node) {
                *mapped = None;
            }
        }
    }

    fn lookup(&self, id: ElementId) -> NodeId {
        self.element_to_node
            .get(id.raw())
            .and_then(|id| *id)
            .filter(|&node| self.arena.get(node).is_some_and(Option::is_some))
            .unwrap_or_else(|| {
                panic!(
                    "renderer asked for unknown ElementId::from_raw({})",
                    id.raw()
                )
            })
    }

    fn pop_nodes(&mut self, m: usize) -> Vec<NodeId> {
        let available = self.stack.len().saturating_sub(1);
        if m > available {
            panic!(
                "renderer stack underflow: tried to pop {m} node(s), only {available} available"
            );
        }
        let split = self.stack.len() - m;
        let _ = self.stack_sources.split_off(split);
        self.stack.split_off(split)
    }

    fn deep_clone_node(&mut self, node: NodeId) -> NodeId {
        let node_data = self.node(node).clone();
        let cloned = self.alloc(match node_data.kind {
            NodeKind::Document => panic!("renderer cannot clone document root"),
            NodeKind::Element { tag, namespace } => NodeKind::Element { tag, namespace },
            NodeKind::Text(text) => NodeKind::Text(text),
        });
        {
            let cloned_node = self.node_mut(cloned);
            cloned_node.attrs = node_data.attrs;
            cloned_node.child_logical_indices = node_data.child_logical_indices;
        }
        let children = node_data
            .children
            .into_iter()
            .map(|child| {
                let cloned_child = self.deep_clone_node(child);
                self.node_mut(cloned_child).parent = Some(cloned);
                cloned_child
            })
            .collect();
        self.node_mut(cloned).children = children;
        cloned
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
        let ti = parent_node.child_logical_indices.remove(index);
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
            parent_node.child_logical_indices.insert(index + offset, ti);
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
            .child_logical_indices
            .extend(std::iter::repeat_n(ti, added));
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
            NodeKind::Text(text) => Some(SnapshotNode::Text(text.clone())),
        }
    }
}

impl WriteMutations for RendererOracle {
    fn push_id(&mut self, id: ElementId) {
        let node = self.lookup(id);
        self.push_stack(node, self.stack_source_for_node(node));
    }

    fn pop_id(&mut self, id: ElementId) {
        let (node, source) = self.pop_stack("pop_id");
        match source {
            StackSource::NewText => self.edit_counters.create_texts += 1,
            StackSource::PrototypeClone => self.edit_counters.loads += 1,
            StackSource::Live | StackSource::PrototypeBuild => {}
        }
        self.set_element_mapping(id, node);
        self.node_roles[node] = if source == StackSource::PrototypeBuild {
            NodeRole::PrototypeRoot
        } else {
            NodeRole::Live
        };
    }

    fn child(&mut self, index: usize) {
        let (parent, source) = self.top_stack("child");
        let child = *self.node(parent).children.get(index).unwrap_or_else(|| {
            panic!("renderer child index {index} out of bounds for node {parent}")
        });
        self.replace_stack_top(child, source, "child");
    }

    fn pop(&mut self) {
        self.pop_stack("pop");
    }

    fn create_element(&mut self, tag: &str, ns: Option<&str>) {
        let node = self.alloc(NodeKind::Element {
            tag: tag.to_string(),
            namespace: ns.map(ToString::to_string),
        });
        self.push_stack(node, StackSource::PrototypeBuild);
    }

    fn create_text(&mut self, value: &str) {
        let node = self.alloc(NodeKind::Text(value.to_string()));
        self.push_stack(node, StackSource::NewText);
    }

    fn clone(&mut self) {
        let (node, _) = self.top_stack("clone");
        let cloned = self.deep_clone_node(node);
        self.replace_stack_top(cloned, StackSource::PrototypeClone, "clone");
    }

    fn append_children(&mut self, m: usize) {
        let parent_source = self.top_stack("append_children").1;
        if parent_source != StackSource::PrototypeBuild {
            self.edit_counters.inserts += 1;
        }
        let nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let parent = self.top_stack("append_children").0;
        self.append_detached(parent, nodes, NO_LOGICAL_INDEX);
    }

    fn replace_with(&mut self, m: usize) {
        self.edit_counters.replaces += 1;
        let nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let (target, _) = self.pop_stack("replace_with");
        let (parent, index, ti) = self.detach(target);
        self.drop_subtree(target);
        self.clear_element_mapping_for_node(target);
        self.insert_detached(parent, index, nodes, ti);
    }

    fn insert_after(&mut self, m: usize) {
        self.edit_counters.inserts += 1;
        let nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let anchor = self.top_stack("insert_after").0;
        let (parent, index) = self.position_in_parent(anchor);
        let ti = self.node(parent).child_logical_indices[index];
        self.insert_detached(parent, index + 1, nodes, ti);
    }

    fn insert_before(&mut self, m: usize) {
        self.edit_counters.inserts += 1;
        let nodes = self.pop_nodes(m);
        self.unhook_all(&nodes);
        let anchor = self.top_stack("insert_before").0;
        let (parent, index) = self.position_in_parent(anchor);
        let ti = self.node(parent).child_logical_indices[index];
        self.insert_detached(parent, index, nodes, ti);
    }

    fn set_attribute(&mut self, name: &str, ns: Option<&str>, value: &AttributeValue) {
        let (node, source) = self.top_stack("set_attribute");
        if source != StackSource::PrototypeBuild {
            self.edit_counters.set_attrs += 1;
        }
        match attr_to_string(value) {
            Some(value) => {
                self.set_attr(node, name.to_string(), ns.map(ToString::to_string), value)
            }
            None => self.remove_attr(node, name, ns),
        }
    }

    fn set_text(&mut self, value: &str) {
        self.edit_counters.set_texts += 1;
        let node = self.top_stack("set_text").0;
        match &mut self.node_mut(node).kind {
            NodeKind::Text(text) => *text = value.to_string(),
            other => panic!("set_text expected text node, got {other:?}"),
        }
    }

    fn add_event_listener(&mut self, name: &str) {
        let node = self.top_stack("add_event_listener").0;
        self.assert_element(node, "add_event_listener");
        let id = self
            .element_id_for_node(node)
            .unwrap_or_else(|| panic!("event listener target node {node} has no ElementId"));
        let target = EventListenerTarget {
            name: name.to_string(),
            id,
        };
        if !self.historical_event_listener_targets.contains(&target) {
            self.historical_event_listener_targets.push(target.clone());
        }
        let listeners = &mut self.node_mut(node).listeners;
        let name = name.to_string();
        match listeners.binary_search(&name) {
            Ok(_) => {}
            Err(index) => listeners.insert(index, name),
        }
    }

    fn remove_event_listener(&mut self, name: &str) {
        let node = self.top_stack("remove_event_listener").0;
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

    fn remove(&mut self) {
        self.edit_counters.removes += 1;
        let (node, _) = self.pop_stack("remove");
        if node == self.root {
            panic!("renderer cannot remove document root");
        }
        self.detach(node);
        self.drop_subtree(node);
        self.clear_element_mapping_for_node(node);
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

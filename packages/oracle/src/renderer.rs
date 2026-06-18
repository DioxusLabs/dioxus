use crate::snapshot::{
    SnapshotAttr, SnapshotNode, attr_key, attr_to_string, format_snapshot_mismatch,
};
use crate::vdom_snapshot::{fresh_snapshot, vdom_snapshot};
use dioxus_core::{AttributeValue, Element, ElementId, VirtualDom, WriteMutations};
use dioxus_stack::{RealDom, StackState, StackWriter};
use std::fmt;

type NodeId = usize;

/// The provenance of a node, used only to categorize edits (see [`EditSummary`]).
///
/// A node built as part of a reusable template prototype is a `PrototypeRoot`;
/// everything else is `Live`. This drives nothing in the tree itself — it only
/// lets the edit counters distinguish "assembling a prototype" from "patching
/// the live tree".
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum NodeRole {
    Live,
    PrototypeRoot,
}

/// The provenance of a node currently on the mutation stack, used only by the
/// edit counters. Mirrors what the renderer stack used to track per entry.
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
    /// The `ElementId` this node is currently mapped to, recorded at `pop_id`.
    /// Lets semantic lookups (`element_id_by_tag`/`_attr`) resolve a tree node
    /// back to its id without core owning a reverse index. Clones start `None`
    /// and get an id only when the diff assigns one.
    element_id: Option<ElementId>,
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventListenerTarget {
    /// The event name registered on the target.
    pub name: String,
    /// The renderer element id that received the listener.
    pub id: ElementId,
}

/// The oracle's in-memory tree — the "real semantics" half of the renderer.
///
/// This implements [`RealDom`] (via [`ArenaBackend`]) and knows nothing about
/// the mutation stack or `ElementId`s; the stack machine lives in core's
/// [`StackWriter`]/[`StackState`] and the edit counters live in
/// [`EditCountingWriter`].
struct OracleArena {
    arena: Vec<Option<Node>>,
    root: NodeId,
    historical_event_listener_targets: Vec<EventListenerTarget>,
}

impl OracleArena {
    fn new() -> Self {
        Self {
            arena: vec![Some(Node {
                kind: NodeKind::Document,
                attrs: Vec::new(),
                listeners: Vec::new(),
                children: Vec::new(),
                child_logical_indices: Vec::new(),
                parent: None,
                element_id: Some(ElementId::ROOT),
            })],
            root: 0,
            historical_event_listener_targets: Vec::new(),
        }
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
            element_id: None,
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

    fn nth_child(&self, parent: NodeId, index: usize) -> NodeId {
        *self.node(parent).children.get(index).unwrap_or_else(|| {
            panic!("renderer child index {index} out of bounds for node {parent}")
        })
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

    fn insert_detached(&mut self, parent: NodeId, index: usize, nodes: &[NodeId], ti: u8) {
        if index > self.node(parent).children.len() {
            panic!(
                "renderer insertion index {index} out of bounds for parent {parent} with {} children",
                self.node(parent).children.len()
            );
        }
        for &node in nodes {
            self.node_mut(node).parent = Some(parent);
        }
        let parent_node = self.node_mut(parent);
        for (offset, &node) in nodes.iter().enumerate() {
            parent_node.children.insert(index + offset, node);
            parent_node.child_logical_indices.insert(index + offset, ti);
        }
    }

    fn append_detached(&mut self, parent: NodeId, nodes: &[NodeId], ti: u8) {
        for &node in nodes {
            self.node_mut(node).parent = Some(parent);
        }
        let parent_node = self.node_mut(parent);
        parent_node.children.extend_from_slice(nodes);
        parent_node
            .child_logical_indices
            .extend(std::iter::repeat_n(ti, nodes.len()));
    }

    fn drop_subtree(&mut self, node: NodeId) {
        if node == self.root {
            panic!("renderer cannot drop document root");
        }
        let node_data = self.arena[node]
            .take()
            .unwrap_or_else(|| panic!("renderer tried to drop already-dead node {node}"));
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

    fn snapshot(&self) -> Vec<SnapshotNode> {
        self.node(self.root)
            .children
            .iter()
            .filter_map(|&child| self.snapshot_node(child))
            .collect()
    }
}

/// The "real semantics" the dioxus stack machine drives. Borrows an
/// [`OracleArena`] for the duration of one render.
struct ArenaBackend<'a> {
    arena: &'a mut OracleArena,
}

impl RealDom for ArenaBackend<'_> {
    type NodeId = NodeId;

    fn create_element(&mut self, tag: &str, ns: Option<&str>) -> NodeId {
        self.arena.alloc(NodeKind::Element {
            tag: tag.to_string(),
            namespace: ns.map(ToString::to_string),
        })
    }

    fn create_text(&mut self, value: &str) -> NodeId {
        self.arena.alloc(NodeKind::Text(value.to_string()))
    }

    fn deep_clone(&mut self, node: NodeId) -> NodeId {
        self.arena.deep_clone_node(node)
    }

    fn nth_child(&mut self, parent: NodeId, index: usize) -> NodeId {
        self.arena.nth_child(parent, index)
    }

    fn append_children(&mut self, parent: NodeId, children: &[NodeId]) {
        self.arena.unhook_all(children);
        self.arena
            .append_detached(parent, children, NO_LOGICAL_INDEX);
    }

    fn insert_after(&mut self, anchor: NodeId, nodes: &[NodeId]) {
        self.arena.unhook_all(nodes);
        let (parent, index) = self.arena.position_in_parent(anchor);
        let ti = self.arena.node(parent).child_logical_indices[index];
        self.arena.insert_detached(parent, index + 1, nodes, ti);
    }

    fn insert_before(&mut self, anchor: NodeId, nodes: &[NodeId]) {
        self.arena.unhook_all(nodes);
        let (parent, index) = self.arena.position_in_parent(anchor);
        let ti = self.arena.node(parent).child_logical_indices[index];
        self.arena.insert_detached(parent, index, nodes, ti);
    }

    fn replace(&mut self, target: NodeId, replacements: &[NodeId]) {
        self.arena.unhook_all(replacements);
        let (parent, index, ti) = self.arena.detach(target);
        self.arena.drop_subtree(target);
        self.arena.insert_detached(parent, index, replacements, ti);
    }

    fn remove(&mut self, node: NodeId) {
        if node == self.arena.root {
            panic!("renderer cannot remove document root");
        }
        self.arena.detach(node);
        self.arena.drop_subtree(node);
    }

    fn set_attribute(
        &mut self,
        node: NodeId,
        name: &str,
        ns: Option<&str>,
        value: &AttributeValue,
    ) {
        match attr_to_string(value) {
            Some(value) => {
                self.arena
                    .set_attr(node, name.to_string(), ns.map(ToString::to_string), value)
            }
            None => self.arena.remove_attr(node, name, ns),
        }
    }

    fn set_text(&mut self, node: NodeId, value: &str) {
        match &mut self.arena.node_mut(node).kind {
            NodeKind::Text(text) => *text = value.to_string(),
            other => panic!("set_text expected text node, got {other:?}"),
        }
    }

    fn add_event_listener(&mut self, node: NodeId, element_id: ElementId, name: &str) {
        self.arena.assert_element(node, "add_event_listener");
        let target = EventListenerTarget {
            name: name.to_string(),
            id: element_id,
        };
        if !self
            .arena
            .historical_event_listener_targets
            .contains(&target)
        {
            self.arena.historical_event_listener_targets.push(target);
        }
        let listeners = &mut self.arena.node_mut(node).listeners;
        let name = name.to_string();
        match listeners.binary_search(&name) {
            Ok(_) => {}
            Err(index) => listeners.insert(index, name),
        }
    }

    fn remove_event_listener(&mut self, node: NodeId, _element_id: ElementId, name: &str) {
        self.arena.assert_element(node, "remove_event_listener");
        let listeners = &mut self.arena.node_mut(node).listeners;
        let name = name.to_string();
        match listeners.binary_search(&name) {
            Ok(index) => {
                listeners.remove(index);
            }
            Err(_) => panic!("renderer removed missing event listener {name:?}"),
        }
    }
}

/// Wraps an inner [`WriteMutations`] writer and reproduces [`EditSummary`] from
/// the mutation stream alone.
///
/// It needs no tree access: a node's provenance is a pure function of the stack
/// ops, tracked here as a shadow source stack plus a per-`ElementId` role map
/// (the role of whatever node is mapped to an id, updated at `pop_id`). This is
/// the same information the renderer stack used to carry, kept out of the tree
/// backend so [`RealDom`] stays pure tree operations.
struct EditCountingWriter<'a, W: WriteMutations> {
    summary: &'a mut EditSummary,
    source_stack: &'a mut Vec<StackSource>,
    roles: &'a mut Vec<NodeRole>,
    inner: W,
}

impl<W: WriteMutations> EditCountingWriter<'_, W> {
    fn role(&self, id: ElementId) -> NodeRole {
        self.roles.get(id.raw()).copied().unwrap_or(NodeRole::Live)
    }

    fn set_role(&mut self, id: ElementId, role: NodeRole) {
        if self.roles.len() <= id.raw() {
            self.roles.resize(id.raw() + 1, NodeRole::Live);
        }
        self.roles[id.raw()] = role;
    }

    fn top_source(&self, op: &str) -> StackSource {
        *self
            .source_stack
            .last()
            .unwrap_or_else(|| panic!("renderer source stack unexpectedly empty during {op}"))
    }

    fn pop_source(&mut self, op: &str) -> StackSource {
        self.source_stack
            .pop()
            .unwrap_or_else(|| panic!("renderer source stack unexpectedly empty during {op}"))
    }

    fn pop_sources(&mut self, m: usize) {
        let split = self.source_stack.len() - m;
        self.source_stack.truncate(split);
    }
}

impl<W: WriteMutations> WriteMutations for EditCountingWriter<'_, W> {
    fn push_id(&mut self, id: ElementId) {
        let source = match self.role(id) {
            NodeRole::Live => StackSource::Live,
            NodeRole::PrototypeRoot => StackSource::PrototypeBuild,
        };
        self.source_stack.push(source);
        self.inner.push_id(id);
    }

    fn pop_id(&mut self, id: ElementId) {
        let source = self.pop_source("pop_id");
        match source {
            StackSource::NewText => self.summary.create_texts += 1,
            StackSource::PrototypeClone => self.summary.loads += 1,
            StackSource::Live | StackSource::PrototypeBuild => {}
        }
        self.set_role(
            id,
            if source == StackSource::PrototypeBuild {
                NodeRole::PrototypeRoot
            } else {
                NodeRole::Live
            },
        );
        self.inner.pop_id(id);
    }

    fn child(&mut self, index: usize) {
        // The selected child keeps the current top's source, so the shadow stack
        // is unchanged.
        self.inner.child(index);
    }

    fn pop(&mut self) {
        self.pop_source("pop");
        self.inner.pop();
    }

    fn create_element(&mut self, tag: &str, ns: Option<&str>) {
        self.source_stack.push(StackSource::PrototypeBuild);
        self.inner.create_element(tag, ns);
    }

    fn create_text(&mut self, value: &str) {
        self.source_stack.push(StackSource::NewText);
        self.inner.create_text(value);
    }

    fn clone(&mut self) {
        *self
            .source_stack
            .last_mut()
            .expect("renderer source stack unexpectedly empty during clone") =
            StackSource::PrototypeClone;
        WriteMutations::clone(&mut self.inner);
    }

    fn append_children(&mut self, m: usize) {
        if self.top_source("append_children") != StackSource::PrototypeBuild {
            self.summary.inserts += 1;
        }
        self.pop_sources(m);
        self.inner.append_children(m);
    }

    fn replace_with(&mut self, m: usize) {
        self.summary.replaces += 1;
        self.pop_sources(m);
        self.pop_source("replace_with");
        self.inner.replace_with(m);
    }

    fn insert_after(&mut self, m: usize) {
        self.summary.inserts += 1;
        self.pop_sources(m);
        self.inner.insert_after(m);
    }

    fn insert_before(&mut self, m: usize) {
        self.summary.inserts += 1;
        self.pop_sources(m);
        self.inner.insert_before(m);
    }

    fn set_attribute(&mut self, name: &str, ns: Option<&str>, value: &AttributeValue) {
        if self.top_source("set_attribute") != StackSource::PrototypeBuild {
            self.summary.set_attrs += 1;
        }
        self.inner.set_attribute(name, ns, value);
    }

    fn set_text(&mut self, value: &str) {
        self.summary.set_texts += 1;
        self.inner.set_text(value);
    }

    fn add_event_listener(&mut self, name: &str) {
        self.inner.add_event_listener(name);
    }

    fn remove_event_listener(&mut self, name: &str) {
        self.inner.remove_event_listener(name);
    }

    fn remove(&mut self) {
        self.summary.removes += 1;
        self.pop_source("remove");
        self.inner.remove();
    }
}

/// A fast in-memory renderer for testing Dioxus mutations.
///
/// `RendererOracle` applies the same mutation stream a renderer receives and
/// exposes stable snapshots that tests can compare against a fresh render.
/// It is intended for unit tests, fuzzing, and focused renderer-behavior
/// assertions that should not require a browser or native window.
pub struct RendererOracle {
    arena: OracleArena,
    state: StackState<NodeId>,
    edit_counters: EditSummary,
    /// Shadow of the renderer stack's per-node provenance, used only for edit
    /// counting. Naturally back to `[Live]` (just the root) between balanced
    /// renders.
    source_stack: Vec<StackSource>,
    /// Per-`ElementId` provenance of the mapped node, used to classify pushes.
    roles: Vec<NodeRole>,
}

impl Default for RendererOracle {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererOracle {
    /// Create an empty document with `ElementId::from_raw(0)` mapped to the document root.
    pub fn new() -> Self {
        let arena = OracleArena::new();
        let root = arena.root;
        Self {
            arena,
            state: StackState::new(root),
            edit_counters: EditSummary::default(),
            source_stack: vec![StackSource::Live],
            roles: Vec::new(),
        }
    }

    /// Build a writer that applies dioxus mutations into the arena, driving the
    /// core-owned stack machine and counting edits.
    fn writer(&mut self) -> EditCountingWriter<'_, StackWriter<'_, ArenaBackend<'_>>> {
        EditCountingWriter {
            summary: &mut self.edit_counters,
            source_stack: &mut self.source_stack,
            roles: &mut self.roles,
            inner: StackWriter::new(
                &mut self.state,
                ArenaBackend {
                    arena: &mut self.arena,
                },
            ),
        }
    }

    /// Return a category-level summary of the edits applied during the most
    /// recent `rebuild` / `render` / `wait_and_render` call. See [`EditSummary`].
    pub fn last_edit_summary(&self) -> EditSummary {
        self.edit_counters.clone()
    }

    /// Return every event listener target attached since the last clear/rebuild.
    pub fn historical_event_listener_targets(&self) -> &[EventListenerTarget] {
        &self.arena.historical_event_listener_targets
    }

    /// Return the live [`ElementId`] mapped to the current stack node.
    pub fn current_stack_element_id(&self) -> Option<ElementId> {
        self.state.current_top_element_id()
    }

    /// Remove all nodes and reset the renderer to an empty document.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Return a stable snapshot of the document root's children.
    pub fn snapshot(&self) -> Vec<SnapshotNode> {
        self.arena.snapshot()
    }

    /// Return the number of non-document nodes currently left on the mutation stack.
    pub fn pending_stack_nodes(&self) -> usize {
        self.state.stack_depth().saturating_sub(1)
    }

    /// Return true when no mutation-created nodes are left on the stack.
    pub fn is_stack_clean(&self) -> bool {
        self.state.stack_depth() == 1
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
        self.collect_element_ids_by_tag(self.arena.root, tag, &mut hits);
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
        self.collect_element_ids_by_attr(self.arena.root, attr_name, attr_value, &mut hits);
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
        let n = self.arena.node(node);
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
        let n = self.arena.node(node);
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
        self.arena.node(node).element_id
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
        self.collect_identities_by_attr(self.arena.root, attr_name, &mut out);
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn collect_identities_by_attr(
        &self,
        node: NodeId,
        attr_name: &str,
        out: &mut Vec<(String, OracleNodeId)>,
    ) {
        let n = self.arena.node(node);
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
    /// directly, without going through any `WriteMutations` path.
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
}

macro_rules! forward_oracle_mutations {
    ($($method:ident($($arg:ident: $arg_ty:ty),*);)*) => {
        $(
            fn $method(&mut self, $($arg: $arg_ty),*) {
                self.writer().$method($($arg),*);
            }
        )*
    };
}

impl WriteMutations for RendererOracle {
    forward_oracle_mutations! {
        push_id(id: ElementId);
        child(index: usize);
        pop();
        create_element(tag: &str, ns: Option<&str>);
        create_text(value: &str);
        append_children(m: usize);
        replace_with(m: usize);
        insert_after(m: usize);
        insert_before(m: usize);
        set_attribute(name: &str, ns: Option<&str>, value: &AttributeValue);
        set_text(value: &str);
        add_event_listener(name: &str);
        remove_event_listener(name: &str);
        remove();
    }

    fn pop_id(&mut self, id: ElementId) {
        self.writer().pop_id(id);
        // Record the id on the node core just mapped it to, so semantic lookups
        // can resolve node -> ElementId without core owning a reverse index.
        if let Some(node) = self.state.element_to_node(id) {
            self.arena.node_mut(node).element_id = Some(id);
        }
    }

    fn clone(&mut self) {
        WriteMutations::clone(&mut self.writer());
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

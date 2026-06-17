//! A reusable stack machine for renderers that materialize a Rust-side tree.
//!
//! [`WriteMutations`] is a low-level stack machine: the diff addresses nodes by
//! their position on a renderer-owned stack rather than by [`ElementId`].
//! Renderers that forward the protocol to another runtime (the web/desktop
//! interpreters forward it to JavaScript) keep that stack on the far side. But
//! renderers that materialize a real Rust tree — the blitz-backed native
//! renderer and the in-memory fuzzing oracle — each used to hand-roll the same
//! bookkeeping: a node stack, an `ElementId -> node` map, and the translation of
//! every stack op into an explicit-node tree operation.
//!
//! This crate factors that out. A renderer implements [`RealDom`] (the "real
//! semantics" — the operations that actually touch its tree, addressed by
//! explicit node handle) and lets [`StackWriter`] provide its
//! [`WriteMutations`] implementation. The stack and mapping live in
//! [`StackState`], which persists across renders.

use dioxus_core::{AttributeValue, ElementId, WriteMutations};

/// A renderer's real-DOM tree, addressed by explicit node handle.
///
/// This is the "real semantics" half of mutation handling: the operations that
/// actually create, move, and mutate nodes in a renderer's tree. The stack
/// machine bookkeeping the diff speaks — pushing and popping nodes, resolving
/// [`ElementId`]s, navigating to children — is handled once by [`StackWriter`],
/// which drives this trait.
pub trait RealDom {
    /// A handle to a node in the renderer's tree. Cheap to copy.
    type NodeId: Copy;

    /// Create a new element node and return its handle.
    fn create_element(&mut self, tag: &str, ns: Option<&str>) -> Self::NodeId;

    /// Create a new text node and return its handle.
    fn create_text(&mut self, value: &str) -> Self::NodeId;

    /// Deep-clone `node` together with its subtree and return the clone.
    fn deep_clone(&mut self, node: Self::NodeId) -> Self::NodeId;

    /// Return the handle of `parent`'s child at `index`.
    fn nth_child(&mut self, parent: Self::NodeId, index: usize) -> Self::NodeId;

    /// Append `children` to the end of `parent`'s child list.
    fn append_children(&mut self, parent: Self::NodeId, children: &[Self::NodeId]);

    /// Insert `nodes` immediately after `anchor` among its siblings.
    fn insert_after(&mut self, anchor: Self::NodeId, nodes: &[Self::NodeId]);

    /// Insert `nodes` immediately before `anchor` among its siblings.
    fn insert_before(&mut self, anchor: Self::NodeId, nodes: &[Self::NodeId]);

    /// Replace `target` together with its subtree in place with `replacements`.
    fn replace(&mut self, target: Self::NodeId, replacements: &[Self::NodeId]);

    /// Remove `node` together with its subtree from the tree.
    fn remove(&mut self, node: Self::NodeId);

    /// Set `node`'s attribute, or remove it when `value` is [`AttributeValue::None`].
    fn set_attribute(
        &mut self,
        node: Self::NodeId,
        name: &str,
        ns: Option<&str>,
        value: &AttributeValue,
    );

    /// Set `node`'s text content.
    fn set_text(&mut self, node: Self::NodeId, value: &str);

    /// Attach an event listener to `node`.
    ///
    /// `element_id` is the [`ElementId`] mapped to the listener target (the
    /// current stack top), which renderers use to route events back to the
    /// virtual DOM.
    fn add_event_listener(&mut self, node: Self::NodeId, element_id: ElementId, name: &str);

    /// Remove an event listener from `node`.
    fn remove_event_listener(&mut self, node: Self::NodeId, element_id: ElementId, name: &str);
}

/// One node on the renderer stack, tagged with the [`ElementId`] it was pushed
/// under (if any) so the mapping can be cleared when the node is removed or
/// replaced without a reverse scan.
#[derive(Clone, Copy, Debug)]
struct StackEntry<N> {
    node: N,
    element_id: Option<ElementId>,
}

/// Persistent stack-machine state shared by renderers that drive a [`RealDom`].
///
/// Holds the renderer's node stack and the `ElementId -> node` mapping. It lives
/// across renders: the diff streams mutations into a fresh [`StackWriter`] each
/// render, but the stack and mapping persist here.
#[derive(Debug)]
pub struct StackState<N> {
    stack: Vec<StackEntry<N>>,
    element_to_node: Vec<Option<N>>,
}

impl<N: Copy> StackState<N> {
    /// Create stack state seeded with `root` mapped to [`ElementId::ROOT`].
    pub fn new(root: N) -> Self {
        Self {
            stack: vec![StackEntry {
                node: root,
                element_id: Some(ElementId::ROOT),
            }],
            element_to_node: vec![Some(root)],
        }
    }

    /// The node currently mapped to `id`, if any.
    pub fn element_to_node(&self, id: ElementId) -> Option<N> {
        self.element_to_node.get(id.raw()).copied().flatten()
    }

    /// The [`ElementId`] of the current top-of-stack node, if it carries one.
    pub fn current_top_element_id(&self) -> Option<ElementId> {
        self.stack.last().and_then(|entry| entry.element_id)
    }

    /// The number of nodes currently on the stack, including the root.
    ///
    /// A balanced render leaves only the root, so `stack_depth() == 1` means the
    /// stack is clean.
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }

    fn lookup(&self, id: ElementId) -> N {
        self.element_to_node(id)
            .unwrap_or_else(|| panic!("renderer asked for unknown ElementId {}", id.raw()))
    }

    fn set_mapping(&mut self, id: ElementId, node: N) {
        let index = id.raw();
        if self.element_to_node.len() <= index {
            self.element_to_node.resize(index + 1, None);
        }
        self.element_to_node[index] = Some(node);
    }

    fn clear_mapping(&mut self, entry: StackEntry<N>) {
        if let Some(id) = entry.element_id
            && let Some(slot) = self.element_to_node.get_mut(id.raw())
        {
            *slot = None;
        }
    }

    fn push(&mut self, node: N, element_id: Option<ElementId>) {
        self.stack.push(StackEntry { node, element_id });
    }

    fn pop_entry(&mut self) -> StackEntry<N> {
        self.stack.pop().expect("renderer stack unexpectedly empty")
    }

    fn top(&self) -> StackEntry<N> {
        *self
            .stack
            .last()
            .expect("renderer stack unexpectedly empty")
    }

    fn replace_top(&mut self, node: N, element_id: Option<ElementId>) {
        *self
            .stack
            .last_mut()
            .expect("renderer stack unexpectedly empty") = StackEntry { node, element_id };
    }

    fn pop_nodes(&mut self, m: usize) -> Vec<N> {
        let split = self.stack.len() - m;
        self.stack
            .split_off(split)
            .into_iter()
            .map(|entry| entry.node)
            .collect()
    }
}

/// A per-render writer that drives a [`RealDom`] backend from the stack
/// protocol.
///
/// Borrows the persistent [`StackState`] and owns (or borrows, for transient
/// backends like a blitz `DocumentMutator`) the backend. This is the single,
/// shared implementation of the stack machine — renderers implement only the
/// [`RealDom`] tree operations.
pub struct StackWriter<'a, R: RealDom> {
    state: &'a mut StackState<R::NodeId>,
    backend: R,
}

impl<'a, R: RealDom> StackWriter<'a, R> {
    /// Drive `backend` from `state` for the duration of one render pass.
    pub fn new(state: &'a mut StackState<R::NodeId>, backend: R) -> Self {
        Self { state, backend }
    }
}

impl<R: RealDom> WriteMutations for StackWriter<'_, R> {
    fn push_id(&mut self, id: ElementId) {
        let node = self.state.lookup(id);
        self.state.push(node, Some(id));
    }

    fn pop_id(&mut self, id: ElementId) {
        let entry = self.state.pop_entry();
        self.state.set_mapping(id, entry.node);
    }

    fn child(&mut self, index: usize) {
        let parent = self.state.top().node;
        let child = self.backend.nth_child(parent, index);
        self.state.replace_top(child, None);
    }

    fn pop(&mut self) {
        self.state.pop_entry();
    }

    fn create_element(&mut self, tag: &str, ns: Option<&str>) {
        let node = self.backend.create_element(tag, ns);
        self.state.push(node, None);
    }

    fn create_text(&mut self, value: &str) {
        let node = self.backend.create_text(value);
        self.state.push(node, None);
    }

    fn clone(&mut self) {
        let node = self.state.top().node;
        let cloned = self.backend.deep_clone(node);
        self.state.replace_top(cloned, None);
    }

    fn append_children(&mut self, m: usize) {
        let children = self.state.pop_nodes(m);
        let parent = self.state.top().node;
        self.backend.append_children(parent, &children);
    }

    fn replace_with(&mut self, m: usize) {
        let replacements = self.state.pop_nodes(m);
        let target = self.state.pop_entry();
        self.backend.replace(target.node, &replacements);
        self.state.clear_mapping(target);
    }

    fn insert_after(&mut self, m: usize) {
        let nodes = self.state.pop_nodes(m);
        let anchor = self.state.top().node;
        self.backend.insert_after(anchor, &nodes);
    }

    fn insert_before(&mut self, m: usize) {
        let nodes = self.state.pop_nodes(m);
        let anchor = self.state.top().node;
        self.backend.insert_before(anchor, &nodes);
    }

    fn set_attribute(&mut self, name: &str, ns: Option<&str>, value: &AttributeValue) {
        let node = self.state.top().node;
        self.backend.set_attribute(node, name, ns, value);
    }

    fn set_text(&mut self, value: &str) {
        let node = self.state.top().node;
        self.backend.set_text(node, value);
    }

    fn add_event_listener(&mut self, name: &str) {
        let entry = self.state.top();
        let element_id = entry
            .element_id
            .expect("event listener target must be mapped to an ElementId");
        self.backend
            .add_event_listener(entry.node, element_id, name);
    }

    fn remove_event_listener(&mut self, name: &str) {
        let entry = self.state.top();
        let element_id = entry
            .element_id
            .expect("event listener target must be mapped to an ElementId");
        self.backend
            .remove_event_listener(entry.node, element_id, name);
    }

    fn remove(&mut self) {
        let entry = self.state.pop_entry();
        self.backend.remove(entry.node);
        self.state.clear_mapping(entry);
    }
}

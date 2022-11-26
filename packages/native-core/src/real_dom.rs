use anymap::AnyMap;
use dioxus_core::{ElementId, Mutations};
use rustc_hash::{FxHashMap, FxHashSet};
use std::ops::{Index, IndexMut};

use crate::node::{Node, NodeData, NodeType, OwnedAttributeDiscription};
use crate::node_ref::{AttributeMask, NodeMask};
use crate::state::State;
use crate::tree::{NodeId, Tree, TreeLike, TreeView};
use crate::RealNodeId;

fn mark_dirty(
    node_id: NodeId,
    mask: NodeMask,
    nodes_updated: &mut FxHashMap<RealNodeId, NodeMask>,
) {
    if let Some(node) = nodes_updated.get_mut(&node_id) {
        *node = node.union(&mask);
    } else {
        nodes_updated.insert(node_id, mask);
    }
}

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [crate::state::ParentDepState], [crate::state::NodeDepState], or [crate::state::ChildDepState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
#[derive(Debug)]
pub struct RealDom<S: State> {
    tree: Tree<Node<S>>,
    /// a map from element id to real node id
    node_id_mapping: Vec<Option<RealNodeId>>,
    nodes_listening: FxHashMap<String, FxHashSet<RealNodeId>>,
    stack: Vec<RealNodeId>,
    templates: FxHashMap<String, Vec<RealNodeId>>,
}

impl<S: State> Default for RealDom<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: State> RealDom<S> {
    pub fn new() -> RealDom<S> {
        let mut root = Node::new(NodeType::Element {
            tag: "Root".to_string(),
            namespace: Some("Root".to_string()),
            attributes: FxHashMap::default(),
            listeners: FxHashSet::default(),
        });
        root.node_data.element_id = Some(ElementId(0));
        let mut tree = Tree::new(root);
        let root_id = tree.root();
        tree.get_mut(root_id).unwrap().node_data.node_id = root_id;

        RealDom {
            tree,
            node_id_mapping: Vec::new(),
            nodes_listening: FxHashMap::default(),
            stack: vec![root_id],
            templates: FxHashMap::default(),
        }
    }

    fn element_to_node_id(&self, element_id: ElementId) -> RealNodeId {
        self.node_id_mapping.get(element_id.0).unwrap().unwrap()
    }

    fn set_element_id(&mut self, node_id: NodeId, element_id: ElementId) {
        let node = self.tree.get_mut(node_id).unwrap();
        let node_id = node.node_data.node_id;
        node.node_data.element_id = Some(element_id);
        if self.node_id_mapping.len() <= element_id.0 {
            self.node_id_mapping.resize(element_id.0 + 1, None);
        }
        self.node_id_mapping[element_id.0] = Some(node_id);
    }

    fn load_child(&self, path: &[u8]) -> RealNodeId {
        let mut current = *self.stack.last().unwrap();
        for i in path {
            current = self.tree.children_ids(current).unwrap()[*i as usize];
        }
        current
    }

    fn create_node(&mut self, node: Node<S>) -> RealNodeId {
        let node_id = self.tree.create_node(node);
        let node = self.tree.get_mut(node_id).unwrap();
        node.node_data.node_id = node_id;
        node_id
    }

    fn add_child(&mut self, node_id: RealNodeId, child_id: RealNodeId) {
        self.tree.add_child(node_id, child_id);
    }

    /// Updates the dom with some mutations and return a set of nodes that were updated. Pass the dirty nodes to update_state.
    pub fn apply_mutations(
        &mut self,
        mutations_vec: Vec<Mutations>,
    ) -> FxHashMap<RealNodeId, NodeMask> {
        let mut nodes_updated: FxHashMap<RealNodeId, NodeMask> = FxHashMap::default();
        for mutations in mutations_vec {
            for e in mutations.edits {
                use dioxus_core::Mutation::*;
                match e {
                    AppendChildren { m } => {
                        let data = self.stack.split_off(m);
                        let (parent, children) = data.split_first().unwrap();
                        for child in children {
                            self.add_child(*parent, *child);
                            mark_dirty(*parent, NodeMask::ALL, &mut nodes_updated);
                        }
                    }
                    AssignId { path, id } => {
                        self.set_element_id(self.load_child(path), id);
                    }
                    CreateElement { name } => {
                        let node = Node::new(NodeType::Element {
                            tag: name.to_string(),
                            namespace: None,
                            attributes: FxHashMap::default(),
                            listeners: FxHashSet::default(),
                        });
                        let id = self.create_node(node);
                        self.stack.push(id);
                        mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                    }
                    CreateElementNamespace { name, namespace } => {
                        let node = Node::new(NodeType::Element {
                            tag: name.to_string(),
                            namespace: Some(namespace.to_string()),
                            attributes: FxHashMap::default(),
                            listeners: FxHashSet::default(),
                        });
                        let id = self.create_node(node);
                        self.stack.push(id);
                        mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                    }
                    CreatePlaceholder { id } => {
                        let node = Node::new(NodeType::Placeholder);
                        let node_id = self.create_node(node);
                        self.set_element_id(node_id, id);
                        self.stack.push(node_id);
                        mark_dirty(node_id, NodeMask::ALL, &mut nodes_updated);
                    }
                    CreateStaticPlaceholder => {
                        let node = Node::new(NodeType::Placeholder);
                        let id = self.create_node(node);
                        self.stack.push(id);
                        mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                    }
                    CreateStaticText { value } => {
                        let node = Node::new(NodeType::Text {
                            text: value.to_string(),
                        });
                        let id = self.create_node(node);
                        self.stack.push(id);
                        mark_dirty(id, NodeMask::new().with_text(), &mut nodes_updated);
                    }
                    CreateTextNode { value, id } => {
                        let node = Node::new(NodeType::Text {
                            text: value.to_string(),
                        });
                        let node_id = self.create_node(node);
                        let node = self.tree.get_mut(node_id).unwrap();
                        node.node_data.element_id = Some(id);
                        self.stack.push(node_id);
                        mark_dirty(node_id, NodeMask::new().with_text(), &mut nodes_updated);
                    }
                    HydrateText { path, value, id } => {
                        let node_id = self.load_child(path);
                        self.set_element_id(node_id, id);
                        let node = self.tree.get_mut(node_id).unwrap();
                        if let NodeType::Text { text } = &mut node.node_data.node_type {
                            *text = value.to_string();
                        }
                        mark_dirty(node_id, NodeMask::new().with_text(), &mut nodes_updated);
                    }
                    LoadTemplate { name, index, id } => {
                        let template_id = self.templates[name][index];
                        let clone_id = self.clone_node(template_id);
                        self.set_element_id(clone_id, id);
                    }
                    ReplaceWith { id, m } => {
                        let new_nodes = self.stack.split_off(m);
                        let old_node_id = self.element_to_node_id(id);
                        for new in new_nodes {
                            self.tree.insert_after(old_node_id, new);
                            mark_dirty(new, NodeMask::ALL, &mut nodes_updated);
                        }
                        self.tree.remove(old_node_id);
                    }
                    ReplacePlaceholder { path, m } => {
                        let new_nodes = self.stack.split_off(m);
                        let old_node_id = self.load_child(path);
                        for new in new_nodes {
                            self.tree.insert_after(old_node_id, new);
                            mark_dirty(new, NodeMask::ALL, &mut nodes_updated);
                        }
                        self.tree.remove(old_node_id);
                    }
                    InsertAfter { id, m } => {
                        let new_nodes = self.stack.split_off(m);
                        let old_node_id = self.element_to_node_id(id);
                        for new in new_nodes {
                            self.tree.insert_after(old_node_id, new);
                            mark_dirty(new, NodeMask::ALL, &mut nodes_updated);
                        }
                    }
                    InsertBefore { id, m } => {
                        let new_nodes = self.stack.split_off(m);
                        let old_node_id = self.element_to_node_id(id);
                        for new in new_nodes {
                            self.tree.insert_before(old_node_id, new);
                            mark_dirty(new, NodeMask::ALL, &mut nodes_updated);
                        }
                    }
                    SaveTemplate { name, m } => {
                        let template = self.stack.split_off(m);
                        self.templates.insert(name.to_string(), template);
                    }
                    SetAttribute {
                        name,
                        value,
                        id,
                        ns,
                    } => {
                        let node_id = self.element_to_node_id(id);
                        let node = self.tree.get_mut(node_id).unwrap();
                        if let NodeType::Element { attributes, .. } = &mut node.node_data.node_type
                        {
                            attributes.insert(
                                OwnedAttributeDiscription {
                                    name: name.to_string(),
                                    namespace: ns.map(|s| s.to_string()),
                                    volatile: false,
                                },
                                crate::node::OwnedAttributeValue::Text(value.to_string()),
                            );
                            mark_dirty(
                                node_id,
                                NodeMask::new_with_attrs(AttributeMask::single(name)),
                                &mut nodes_updated,
                            );
                        }
                    }
                    SetStaticAttribute { name, value, ns } => {
                        let node_id = self.stack.last().unwrap();
                        let node = self.tree.get_mut(*node_id).unwrap();
                        if let NodeType::Element { attributes, .. } = &mut node.node_data.node_type
                        {
                            attributes.insert(
                                OwnedAttributeDiscription {
                                    name: name.to_string(),
                                    namespace: ns.map(|s| s.to_string()),
                                    volatile: false,
                                },
                                crate::node::OwnedAttributeValue::Text(value.to_string()),
                            );
                            mark_dirty(
                                *node_id,
                                NodeMask::new_with_attrs(AttributeMask::single(name)),
                                &mut nodes_updated,
                            );
                        }
                    }
                    SetBoolAttribute { name, value, id } => {
                        let node_id = self.element_to_node_id(id);
                        let node = self.tree.get_mut(node_id).unwrap();
                        if let NodeType::Element { attributes, .. } = &mut node.node_data.node_type
                        {
                            attributes.insert(
                                OwnedAttributeDiscription {
                                    name: name.to_string(),
                                    namespace: None,
                                    volatile: false,
                                },
                                crate::node::OwnedAttributeValue::Bool(value),
                            );
                            mark_dirty(
                                node_id,
                                NodeMask::new_with_attrs(AttributeMask::single(name)),
                                &mut nodes_updated,
                            );
                        }
                    }
                    SetInnerText { value } => {
                        let node_id = *self.stack.last().unwrap();
                        let node = self.tree.get_mut(node_id).unwrap();
                        if let NodeType::Element { .. } = &mut node.node_data.node_type {
                            self.tree.remove_all_children(node_id);
                            let text_node = Node::new(NodeType::Text {
                                text: value.to_string(),
                            });
                            let text_node_id = self.create_node(text_node);
                            self.add_child(node_id, text_node_id);
                        }
                    }
                    SetText { value, id } => {
                        let node_id = self.element_to_node_id(id);
                        let node = self.tree.get_mut(node_id).unwrap();
                        if let NodeType::Text { text } = &mut node.node_data.node_type {
                            *text = value.to_string();
                        }
                        mark_dirty(node_id, NodeMask::new().with_text(), &mut nodes_updated);
                    }
                    NewEventListener {
                        event_name,
                        scope: _,
                        id,
                    } => {
                        let node_id = self.element_to_node_id(id);
                        let node = self.tree.get_mut(node_id).unwrap();
                        if let NodeType::Element { listeners, .. } = &mut node.node_data.node_type {
                            match self.nodes_listening.get_mut(event_name) {
                                Some(hs) => {
                                    hs.insert(node_id);
                                }
                                None => {
                                    let mut hs = FxHashSet::default();
                                    hs.insert(node_id);
                                    self.nodes_listening.insert(event_name.to_string(), hs);
                                }
                            }
                            listeners.insert(event_name.to_string());
                        }
                    }
                    RemoveEventListener { id, event } => {
                        let node_id = self.element_to_node_id(id);
                        let node = self.tree.get_mut(node_id).unwrap();
                        if let NodeType::Element { listeners, .. } = &mut node.node_data.node_type {
                            listeners.remove(event);
                        }
                        self.nodes_listening
                            .get_mut(event)
                            .unwrap()
                            .remove(&node_id);
                    }
                    Remove { id } => {
                        let node_id = self.element_to_node_id(id);
                        self.tree.remove(node_id);
                    }
                }
            }
        }

        // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
        nodes_updated.retain(|k, _| self.tree.get(*k).is_some());

        nodes_updated
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state(
        &mut self,
        nodes_updated: FxHashMap<RealNodeId, NodeMask>,
        ctx: AnyMap,
    ) -> FxHashSet<RealNodeId> {
        let (mut state_tree, node_tree) = self.split();
        S::update(&nodes_updated, &mut state_tree, &node_tree, &ctx)
    }

    /// Find all nodes that are listening for an event, sorted by there height in the dom progressing starting at the bottom and progressing up.
    /// This can be useful to avoid creating duplicate events.
    pub fn get_listening_sorted(&self, event: &'static str) -> Vec<&Node<S>> {
        if let Some(nodes) = self.nodes_listening.get(event) {
            let mut listening: Vec<_> = nodes.iter().map(|id| &self[*id]).collect();
            listening.sort_by(|n1, n2| {
                (self.tree.height(n1.node_data.node_id))
                    .cmp(&self.tree.height(n2.node_data.node_id))
                    .reverse()
            });
            listening
        } else {
            Vec::new()
        }
    }

    /// Return the number of nodes in the dom.
    pub fn size(&self) -> usize {
        // The dom has a root node, ignore it.
        self.tree.size() - 1
    }

    /// Returns the id of the root node.
    pub fn root_id(&self) -> NodeId {
        self.tree.root()
    }

    pub fn split(&mut self) -> (impl TreeView<S> + '_, impl TreeView<NodeData> + '_) {
        let raw = self as *mut Self;
        // this is safe beacuse the treeview trait does not allow mutation of the position of elements, and within elements the access is disjoint.
        (
            unsafe { &mut *raw }
                .tree
                .map(|n| &n.state, |n| &mut n.state),
            unsafe { &mut *raw }
                .tree
                .map(|n| &n.node_data, |n| &mut n.node_data),
        )
    }

    fn clone_node(&mut self, node_id: NodeId) -> RealNodeId {
        let node = self.tree.get(node_id).unwrap();
        let new_node = node.clone();
        let new_id = self.create_node(new_node);
        let self_ptr = self as *mut Self;
        for child in self.tree.children_ids(node_id).unwrap() {
            unsafe {
                // this is safe because no node has itself as a child
                let self_mut = &mut *self_ptr;
                let child_id = self_mut.clone_node(*child);
                self_mut.add_child(new_id, child_id);
            }
        }
        new_id
    }
}

impl<S: State> Index<ElementId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, id: ElementId) -> &Self::Output {
        self.tree.get(self.element_to_node_id(id)).unwrap()
    }
}

impl<S: State> Index<RealNodeId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: RealNodeId) -> &Self::Output {
        self.tree.get(idx).unwrap()
    }
}

impl<S: State> IndexMut<ElementId> for RealDom<S> {
    fn index_mut(&mut self, id: ElementId) -> &mut Self::Output {
        self.tree.get_mut(self.element_to_node_id(id)).unwrap()
    }
}

impl<S: State> IndexMut<RealNodeId> for RealDom<S> {
    fn index_mut(&mut self, idx: RealNodeId) -> &mut Self::Output {
        self.tree.get_mut(idx).unwrap()
    }
}

use dioxus_core::{ElementId, Mutations, TemplateNode};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

use crate::node::{
    ElementNode, Node, NodeData, NodeType, OwnedAttributeDiscription, OwnedAttributeValue,
};
use crate::node_ref::{AttributeMask, NodeMask};
use crate::passes::{resolve_passes, DirtyNodeStates, TypeErasedPass};
use crate::state::State;
use crate::tree::{NodeId, Tree, TreeLike, TreeView, TreeViewMut};
use crate::{FxDashSet, SendAnyMap};

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [crate::state::ParentDepState], [crate::state::NodeDepState], or [crate::state::ChildDepState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
pub struct RealDom<S: State + Send> {
    pub tree: Tree<Node<S>>,
    /// a map from element id to real node id
    node_id_mapping: Vec<Option<NodeId>>,
    nodes_listening: FxHashMap<String, FxHashSet<NodeId>>,
    stack: Vec<NodeId>,
    templates: FxHashMap<String, Vec<NodeId>>,
    pub(crate) passes: Box<[TypeErasedPass<S>]>,
    pub(crate) nodes_updated: FxHashMap<NodeId, NodeMask>,
    passes_updated: DirtyNodeStates,
    parent_changed_nodes: FxHashSet<NodeId>,
    child_changed_nodes: FxHashSet<NodeId>,
    nodes_created: FxHashSet<NodeId>,
}

impl<S: State + Send + Debug> Debug for RealDom<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealDom")
            .field("tree", &self.tree)
            .field("node_id_mapping", &self.node_id_mapping)
            .field("nodes_listening", &self.nodes_listening)
            .field("stack", &self.stack)
            .field("templates", &self.templates)
            .finish()
    }
}

impl<S: State + Send> Default for RealDom<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: State + Send> RealDom<S> {
    pub fn new() -> RealDom<S> {
        let mut root = Node::new(NodeType::Element(ElementNode {
            tag: "Root".to_string(),
            namespace: Some("Root".to_string()),
            attributes: FxHashMap::default(),
            listeners: FxHashSet::default(),
        }));
        root.node_data.element_id = Some(ElementId(0));
        let mut tree = Tree::new(root);
        let root_id = tree.root();
        tree.get_mut(root_id).unwrap().node_data.node_id = root_id;

        let mut passes = S::create_passes();

        // resolve dependants for each pass
        for i in 1..passes.len() {
            let (before, after) = passes.split_at_mut(i);
            let (current, before) = before.split_last_mut().unwrap();
            for pass in before.iter_mut().chain(after.iter_mut()) {
                for dependancy in &current.combined_dependancy_type_ids {
                    if pass.this_type_id == *dependancy {
                        pass.dependants.insert(current.this_type_id);
                    }
                }
            }
        }

        let mut nodes_updated = FxHashMap::default();
        let root_id = NodeId(0);
        nodes_updated.insert(root_id, NodeMask::ALL);

        RealDom {
            tree,
            node_id_mapping: vec![Some(root_id)],
            nodes_listening: FxHashMap::default(),
            stack: vec![root_id],
            templates: FxHashMap::default(),
            passes,
            nodes_updated,
            passes_updated: DirtyNodeStates::default(),
            parent_changed_nodes: FxHashSet::default(),
            child_changed_nodes: FxHashSet::default(),
            nodes_created: FxHashSet::default(),
        }
    }

    fn mark_dirty(&mut self, node_id: NodeId, mask: NodeMask) {
        if let Some(node) = self.nodes_updated.get_mut(&node_id) {
            *node = node.union(&mask);
        } else {
            self.nodes_updated.insert(node_id, mask);
        }
    }

    fn mark_parent_added_or_removed(&mut self, node_id: NodeId) {
        self.parent_changed_nodes.insert(node_id);
    }

    fn mark_child_changed(&mut self, node_id: NodeId) {
        self.child_changed_nodes.insert(node_id);
    }

    pub fn element_to_node_id(&self, element_id: ElementId) -> NodeId {
        self.node_id_mapping.get(element_id.0).unwrap().unwrap()
    }

    fn set_element_id(&mut self, node_id: NodeId, element_id: ElementId) {
        let node = self.tree.get_mut(node_id).unwrap();
        let node_id = node.node_data.node_id;
        node.node_data.element_id = Some(element_id);
        if self.node_id_mapping.len() <= element_id.0 {
            self.node_id_mapping.resize(element_id.0 + 1, None);
        }
        if let Some(Some(old_id)) = self.node_id_mapping.get(element_id.0) {
            // free the memory associated with the old node
            self.tree.remove(*old_id);
        }
        self.node_id_mapping[element_id.0] = Some(node_id);
    }

    fn load_child(&self, path: &[u8]) -> NodeId {
        let mut current = *self.stack.last().unwrap();
        for i in path {
            current = self.tree.children_ids(current).unwrap()[*i as usize];
        }
        current
    }

    fn create_node(&mut self, node: Node<S>, mark_dirty: bool) -> NodeId {
        let node_id = self.tree.create_node(node);
        let node = self.tree.get_mut(node_id).unwrap();
        node.node_data.node_id = node_id;
        if mark_dirty {
            self.nodes_created.insert(node_id);
        }
        node_id
    }

    fn add_child(&mut self, node_id: NodeId, child_id: NodeId) {
        self.tree.add_child(node_id, child_id);
    }

    fn create_template_node(&mut self, node: &TemplateNode) -> NodeId {
        match node {
            TemplateNode::Element {
                tag,
                namespace,
                attrs,
                children,
            } => {
                let node = Node::new(NodeType::Element(ElementNode {
                    tag: tag.to_string(),
                    namespace: namespace.map(|s| s.to_string()),
                    attributes: attrs
                        .iter()
                        .filter_map(|attr| match attr {
                            dioxus_core::TemplateAttribute::Static {
                                name,
                                value,
                                namespace,
                            } => Some((
                                OwnedAttributeDiscription {
                                    namespace: namespace.map(|s| s.to_string()),
                                    name: name.to_string(),
                                    volatile: false,
                                },
                                OwnedAttributeValue::Text(value.to_string()),
                            )),
                            dioxus_core::TemplateAttribute::Dynamic { .. } => None,
                        })
                        .collect(),
                    listeners: FxHashSet::default(),
                }));
                let node_id = self.create_node(node, true);
                for child in *children {
                    let child_id = self.create_template_node(child);
                    self.add_child(node_id, child_id);
                }
                node_id
            }
            TemplateNode::Text { text } => {
                self.create_node(Node::new(NodeType::Text(text.to_string())), true)
            }
            TemplateNode::Dynamic { .. } => {
                self.create_node(Node::new(NodeType::Placeholder), true)
            }
            TemplateNode::DynamicText { .. } => {
                self.create_node(Node::new(NodeType::Text(String::new())), true)
            }
        }
    }

    /// Updates the dom with some mutations and return a set of nodes that were updated. Pass the dirty nodes to update_state.
    pub fn apply_mutations(&mut self, mutations: Mutations) {
        for template in mutations.templates {
            let mut template_root_ids = Vec::new();
            for root in template.roots {
                let id = self.create_template_node(root);
                template_root_ids.push(id);
            }
            self.templates
                .insert(template.name.to_string(), template_root_ids);
        }

        for e in mutations.edits {
            use dioxus_core::Mutation::*;
            match e {
                AppendChildren { id, m } => {
                    let children = self.stack.split_off(self.stack.len() - m);
                    let parent = self.element_to_node_id(id);
                    for child in children {
                        self.add_child(parent, child);
                    }
                }
                AssignId { path, id } => {
                    self.set_element_id(self.load_child(path), id);
                }
                CreatePlaceholder { id } => {
                    let node = Node::new(NodeType::Placeholder);
                    let node_id = self.create_node(node, true);
                    self.set_element_id(node_id, id);
                    self.stack.push(node_id);
                }
                CreateTextNode { value, id } => {
                    let node = Node::new(NodeType::Text(value.to_string()));
                    let node_id = self.create_node(node, true);
                    let node = self.tree.get_mut(node_id).unwrap();
                    node.node_data.element_id = Some(id);
                    self.stack.push(node_id);
                }
                HydrateText { path, value, id } => {
                    let node_id = self.load_child(path);
                    self.set_element_id(node_id, id);
                    let node = self.tree.get_mut(node_id).unwrap();
                    if let NodeType::Text(text) = &mut node.node_data.node_type {
                        *text = value.to_string();
                    } else {
                        node.node_data.node_type = NodeType::Text(value.to_string());
                    }

                    self.mark_dirty(node_id, NodeMask::new().with_text());
                }
                LoadTemplate { name, index, id } => {
                    let template_id = self.templates[name][index];
                    let clone_id = self.clone_node(template_id);
                    self.set_element_id(clone_id, id);
                    self.stack.push(clone_id);
                }
                ReplaceWith { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes {
                        self.tree.insert_before(old_node_id, new);
                    }
                    self.tree.remove(old_node_id);
                }
                ReplacePlaceholder { path, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.load_child(path);
                    for new in new_nodes {
                        self.tree.insert_before(old_node_id, new);
                    }
                    self.tree.remove(old_node_id);
                }
                InsertAfter { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes.into_iter().rev() {
                        self.tree.insert_after(old_node_id, new);
                    }
                }
                InsertBefore { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes {
                        self.tree.insert_before(old_node_id, new);
                    }
                }
                SetAttribute {
                    name,
                    value,
                    id,
                    ns,
                } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = self.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Element(mut element) = node.node_type_mut() {
                        element.set_attribute(
                            OwnedAttributeDiscription {
                                name: name.to_string(),
                                namespace: ns.map(|s| s.to_string()),
                                volatile: false,
                            },
                            crate::node::OwnedAttributeValue::Text(value.to_string()),
                        );
                    }
                }
                SetBoolAttribute { name, value, id } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = self.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Element(mut element) = node.node_type_mut() {
                        element.set_attribute(
                            OwnedAttributeDiscription {
                                name: name.to_string(),
                                namespace: None,
                                volatile: false,
                            },
                            crate::node::OwnedAttributeValue::Bool(value),
                        );
                    }
                }
                SetText { value, id } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = self.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Text(text) = node.node_type_mut() {
                        *text = value.to_string();
                    }
                }
                NewEventListener { name, id } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = self.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Element(mut element) = node.node_type_mut() {
                        element.listeners_mut().insert(name.to_string());
                        drop(node);
                        match self.nodes_listening.get_mut(name) {
                            Some(hs) => {
                                hs.insert(node_id);
                            }
                            None => {
                                let mut hs = FxHashSet::default();
                                hs.insert(node_id);
                                self.nodes_listening.insert(name.to_string(), hs);
                            }
                        }
                    }
                }
                RemoveEventListener { id, name } => {
                    let node_id = self.element_to_node_id(id);
                    {
                        let mut node = self.get_mut(node_id).unwrap();
                        if let NodeTypeMut::Element(mut element) = node.node_type_mut() {
                            element.listeners_mut().remove(name);
                        }
                    }
                    self.nodes_listening.get_mut(name).unwrap().remove(&node_id);
                }
                Remove { id } => {
                    let node_id = self.element_to_node_id(id);
                    self.tree.remove(node_id);
                }
                PushRoot { id } => {
                    let node_id = self.element_to_node_id(id);
                    self.stack.push(node_id);
                }
            }
        }
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state(
        &mut self,
        ctx: SendAnyMap,
    ) -> (FxDashSet<NodeId>, FxHashMap<NodeId, NodeMask>) {
        let passes = &self.passes;
        let dirty_nodes = std::mem::take(&mut self.passes_updated);
        let nodes_updated = std::mem::take(&mut self.nodes_updated);
        for (&node, mask) in &nodes_updated {
            // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
            if self.tree.contains(node) {
                for pass in &*self.passes {
                    if mask.overlaps(&pass.mask) {
                        dirty_nodes.insert(pass.this_type_id, node);
                    }
                }
            }
        }
        for node in std::mem::take(&mut self.child_changed_nodes) {
            // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
            if self.tree.contains(node) {
                for pass in &*self.passes {
                    if pass.child_dependant {
                        dirty_nodes.insert(pass.this_type_id, node);
                    }
                }
            }
        }
        for node in std::mem::take(&mut self.parent_changed_nodes) {
            // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
            if self.tree.contains(node) {
                for pass in &*self.passes {
                    if pass.parent_dependant {
                        dirty_nodes.insert(pass.this_type_id, node);
                    }
                }
            }
        }
        for node in std::mem::take(&mut self.nodes_created) {
            // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
            if self.tree.contains(node) {
                for pass in &*self.passes {
                    dirty_nodes.insert(pass.this_type_id, node);
                }
            }
        }

        let tree = &mut self.tree;

        (
            resolve_passes(tree, dirty_nodes, passes, ctx),
            nodes_updated,
        )
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

    fn clone_node(&mut self, node_id: NodeId) -> NodeId {
        let node = self.tree.get(node_id).unwrap();
        let new_node = Node {
            state: node.state.clone_or_default(),
            node_data: node.node_data.clone(),
        };
        let new_id = self.create_node(new_node, true);

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

    pub fn get_mut(&mut self, id: NodeId) -> Option<NodeMut<'_>> {
        self.tree.get_mut(id).map(|node| NodeMut {
            node: &mut node.node_data,
            dirty: NodeMask::new(),
            nodes_updated: &mut self.nodes_updated,
        })
    }

    /// WARNING: This escapes the reactive system that the real dom uses. Any changes made with this method will not trigger updates in states when [RealDom::update_state] is called.
    pub fn get_mut_raw(&mut self, id: NodeId) -> Option<&mut Node<S>> {
        self.tree.get_mut(id)
    }
}

impl<S: State + Send> TreeView<Node<S>> for RealDom<S> {
    type Iterator<'a> = <Tree<Node<S>> as TreeView<Node<S>>>::Iterator<'a>;

    fn root(&self) -> NodeId {
        self.tree.root()
    }

    fn get(&self, id: NodeId) -> Option<&Node<S>> {
        self.tree.get(id)
    }

    fn children(&self, id: NodeId) -> Option<Self::Iterator<'_>> {
        self.tree.children(id)
    }

    fn children_ids(&self, id: NodeId) -> Option<&[NodeId]> {
        self.tree.children_ids(id)
    }

    fn parent(&self, id: NodeId) -> Option<&Node<S>> {
        self.tree.parent(id)
    }

    fn parent_id(&self, id: NodeId) -> Option<NodeId> {
        self.tree.parent_id(id)
    }

    fn height(&self, id: NodeId) -> Option<u16> {
        self.tree.height(id)
    }

    fn size(&self) -> usize {
        self.tree.size()
    }
}

impl<S: State + Send> TreeLike<Node<S>> for RealDom<S> {
    fn create_node(&mut self, node: Node<S>) -> NodeId {
        let id = self.tree.create_node(node);
        self.tree.get_mut(id).unwrap().node_data.node_id = id;
        self.nodes_created.insert(id);
        id
    }

    fn add_child(&mut self, parent: NodeId, child: NodeId) {
        // mark the parent's children changed
        self.mark_child_changed(parent);
        // mark the child's parent changed
        self.mark_parent_added_or_removed(child);
        self.tree.add_child(parent, child);
    }

    fn remove(&mut self, id: NodeId) -> Option<Node<S>> {
        if let Some(parent_id) = self.tree.parent_id(id) {
            self.mark_child_changed(parent_id);
        }
        self.tree.remove(id)
    }

    fn remove_all_children(&mut self, id: NodeId) -> Vec<Node<S>> {
        self.mark_child_changed(id);
        self.tree.remove_all_children(id)
    }

    fn replace(&mut self, old: NodeId, new: NodeId) {
        if let Some(parent_id) = self.tree.parent_id(old) {
            self.mark_child_changed(parent_id);
            self.mark_parent_added_or_removed(new);
        }
        self.tree.replace(old, new);
    }

    fn insert_before(&mut self, id: NodeId, new: NodeId) {
        if let Some(parent_id) = self.tree.parent_id(id) {
            self.mark_child_changed(parent_id);
            self.mark_parent_added_or_removed(new);
        }
        self.tree.insert_before(id, new);
    }

    fn insert_after(&mut self, id: NodeId, new: NodeId) {
        if let Some(parent_id) = self.tree.parent_id(id) {
            self.mark_child_changed(parent_id);
            self.mark_parent_added_or_removed(new);
        }
        self.tree.insert_after(id, new);
    }
}

impl<S: State + Send> Index<ElementId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, id: ElementId) -> &Self::Output {
        self.tree.get(self.element_to_node_id(id)).unwrap()
    }
}

impl<S: State + Send> Index<NodeId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: NodeId) -> &Self::Output {
        self.tree.get(idx).unwrap()
    }
}

impl<S: State + Send> IndexMut<ElementId> for RealDom<S> {
    fn index_mut(&mut self, id: ElementId) -> &mut Self::Output {
        self.tree.get_mut(self.element_to_node_id(id)).unwrap()
    }
}

impl<S: State + Send> IndexMut<NodeId> for RealDom<S> {
    fn index_mut(&mut self, idx: NodeId) -> &mut Self::Output {
        self.tree.get_mut(idx).unwrap()
    }
}

pub struct NodeMut<'a> {
    node: &'a mut NodeData,
    dirty: NodeMask,
    nodes_updated: &'a mut FxHashMap<NodeId, NodeMask>,
}

impl<'a> NodeMut<'a> {
    pub fn node_type(&self) -> &NodeType {
        &self.node.node_type
    }

    pub fn node_type_mut(&mut self) -> NodeTypeMut<'_> {
        match &mut self.node.node_type {
            NodeType::Element(element) => NodeTypeMut::Element(ElementNodeMut {
                element,
                dirty: &mut self.dirty,
            }),
            NodeType::Text(text) => {
                self.dirty.set_text();
                NodeTypeMut::Text(text)
            }
            NodeType::Placeholder => NodeTypeMut::Placeholder,
        }
    }
}

impl Drop for NodeMut<'_> {
    fn drop(&mut self) {
        let node_id = self.node.node_id;
        let mask = std::mem::take(&mut self.dirty);
        if let Some(node) = self.nodes_updated.get_mut(&node_id) {
            *node = node.union(&mask);
        } else {
            self.nodes_updated.insert(node_id, mask);
        }
    }
}

pub enum NodeTypeMut<'a> {
    Element(ElementNodeMut<'a>),
    Text(&'a mut String),
    Placeholder,
}

pub struct ElementNodeMut<'a> {
    element: &'a mut ElementNode,
    dirty: &'a mut NodeMask,
}

impl ElementNodeMut<'_> {
    pub fn tag(&self) -> &str {
        &self.element.tag
    }

    pub fn tag_mut(&mut self) -> &mut String {
        self.dirty.set_tag();
        &mut self.element.tag
    }

    pub fn namespace(&self) -> Option<&str> {
        self.element.namespace.as_deref()
    }

    pub fn namespace_mut(&mut self) -> &mut Option<String> {
        self.dirty.set_namespace();
        &mut self.element.namespace
    }

    pub fn attributes(&self) -> &FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue> {
        &self.element.attributes
    }

    pub fn attributes_mut(
        &mut self,
    ) -> &mut FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue> {
        self.dirty.add_attributes(AttributeMask::All);
        &mut self.element.attributes
    }

    pub fn set_attribute(
        &mut self,
        name: OwnedAttributeDiscription,
        value: OwnedAttributeValue,
    ) -> Option<OwnedAttributeValue> {
        self.dirty.add_attributes(AttributeMask::single(&name.name));
        self.element.attributes.insert(name, value)
    }

    pub fn get_attribute_mut(
        &mut self,
        name: &OwnedAttributeDiscription,
    ) -> Option<&mut OwnedAttributeValue> {
        self.dirty.add_attributes(AttributeMask::single(&name.name));
        self.element.attributes.get_mut(name)
    }

    pub fn listeners(&self) -> &FxHashSet<String> {
        &self.element.listeners
    }

    pub fn listeners_mut(&mut self) -> &mut FxHashSet<String> {
        self.dirty.set_listeners();
        &mut self.element.listeners
    }
}

use dioxus_core::{BorrowedAttributeValue, ElementId, Mutations, TemplateNode};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::node::{FromAnyValue, Node, NodeType, OwnedAttributeDiscription, OwnedAttributeValue};
use crate::node_ref::{AttributeMask, NodeMask};
use crate::passes::DirtyNodeStates;
use crate::state::State;
use crate::tree::{NodeId, Tree, TreeLike, TreeView};
use crate::{FxDashSet, RealNodeId, SendAnyMap};

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
///
/// # Custom values
/// To allow custom values to be passed into attributes implement FromAnyValue on a type that can represent your custom value and specify the V generic to be that type. If you have many different custom values, it can be useful to use a enum type to represent the varients.
#[derive(Debug)]
pub struct RealDom<S: State<V>, V: FromAnyValue + 'static = ()> {
    pub tree: Tree<Node<S, V>>,
    /// a map from element id to real node id
    node_id_mapping: Vec<Option<RealNodeId>>,
    nodes_listening: FxHashMap<String, FxHashSet<RealNodeId>>,
    stack: Vec<RealNodeId>,
    templates: FxHashMap<String, Vec<RealNodeId>>,
    root_initialized: bool,
}

impl<S: State<V>, V: FromAnyValue> Default for RealDom<S, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: State<V>, V: FromAnyValue> RealDom<S, V> {
    pub fn new() -> RealDom<S, V> {
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
            node_id_mapping: vec![Some(root_id)],
            nodes_listening: FxHashMap::default(),
            stack: vec![root_id],
            templates: FxHashMap::default(),
            root_initialized: false,
        }
    }

    pub fn element_to_node_id(&self, element_id: ElementId) -> RealNodeId {
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

    fn create_node(&mut self, node: Node<S, V>) -> RealNodeId {
        let node_id = self.tree.create_node(node);
        let node = self.tree.get_mut(node_id).unwrap();
        node.node_data.node_id = node_id;
        node_id
    }

    fn add_child(&mut self, node_id: RealNodeId, child_id: RealNodeId) {
        self.tree.add_child(node_id, child_id);
    }

    fn create_template_node(&mut self, node: &TemplateNode) -> RealNodeId {
        match node {
            TemplateNode::Element {
                tag,
                namespace,
                attrs,
                children,
            } => {
                let node = Node::new(NodeType::Element {
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
                });
                let node_id = self.create_node(node);
                for child in *children {
                    let child_id = self.create_template_node(child);
                    self.add_child(node_id, child_id);
                }
                node_id
            }
            TemplateNode::Text { text } => self.create_node(Node::new(NodeType::Text {
                text: text.to_string(),
            })),
            TemplateNode::Dynamic { .. } => self.create_node(Node::new(NodeType::Placeholder)),
            TemplateNode::DynamicText { .. } => self.create_node(Node::new(NodeType::Text {
                text: String::new(),
            })),
        }
    }

    /// Updates the dom with some mutations and return a set of nodes that were updated. Pass the dirty nodes to update_state.
    pub fn apply_mutations(
        &mut self,
        mutations: Mutations,
    ) -> (DirtyNodeStates, FxHashMap<RealNodeId, NodeMask>) {
        let mut nodes_updated: FxHashMap<RealNodeId, NodeMask> = FxHashMap::default();
        for template in mutations.templates {
            let mut template_root_ids = Vec::new();
            for root in template.roots {
                let id = self.create_template_node(root);
                template_root_ids.push(id);
            }
            self.templates
                .insert(template.name.to_string(), template_root_ids);
        }
        if !self.root_initialized {
            self.root_initialized = true;
            let root_id = self.tree.root();
            nodes_updated.insert(root_id, NodeMask::ALL);
        }
        for e in mutations.edits {
            use dioxus_core::Mutation::*;
            match e {
                AppendChildren { id, m } => {
                    let children = self.stack.split_off(self.stack.len() - m);
                    let parent = self.element_to_node_id(id);
                    for child in children {
                        self.add_child(parent, child);
                        mark_dirty(child, NodeMask::ALL, &mut nodes_updated);
                    }
                }
                AssignId { path, id } => {
                    self.set_element_id(self.load_child(path), id);
                }
                CreatePlaceholder { id } => {
                    let node = Node::new(NodeType::Placeholder);
                    let node_id = self.create_node(node);
                    self.set_element_id(node_id, id);
                    self.stack.push(node_id);
                    mark_dirty(node_id, NodeMask::ALL, &mut nodes_updated);
                }
                CreateTextNode { value, id } => {
                    let node = Node::new(NodeType::Text {
                        text: value.to_string(),
                    });
                    let node_id = self.create_node(node);
                    self.set_element_id(node_id, id);
                    self.stack.push(node_id);
                    mark_dirty(node_id, NodeMask::new().with_text(), &mut nodes_updated);
                }
                HydrateText { path, value, id } => {
                    let node_id = self.load_child(path);
                    self.set_element_id(node_id, id);
                    let node = self.tree.get_mut(node_id).unwrap();
                    if let NodeType::Text { text } = &mut node.node_data.node_type {
                        *text = value.to_string();
                    } else {
                        node.node_data.node_type = NodeType::Text {
                            text: value.to_string(),
                        };
                    }

                    mark_dirty(node_id, NodeMask::new().with_text(), &mut nodes_updated);
                }
                LoadTemplate { name, index, id } => {
                    let template_id = self.templates[name][index];
                    let clone_id = self.clone_node(template_id, &mut nodes_updated);
                    self.set_element_id(clone_id, id);
                    self.stack.push(clone_id);
                }
                ReplaceWith { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes {
                        self.tree.insert_before(old_node_id, new);
                        mark_dirty(new, NodeMask::ALL, &mut nodes_updated);
                    }
                    self.remove(old_node_id, &mut nodes_updated);
                }
                ReplacePlaceholder { path, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.load_child(path);
                    for new in new_nodes {
                        self.tree.insert_before(old_node_id, new);
                        mark_dirty(new, NodeMask::ALL, &mut nodes_updated);
                    }
                    self.remove(old_node_id, &mut nodes_updated);
                }
                InsertAfter { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes.into_iter().rev() {
                        self.tree.insert_after(old_node_id, new);
                        mark_dirty(new, NodeMask::ALL, &mut nodes_updated);
                    }
                }
                InsertBefore { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes {
                        self.tree.insert_before(old_node_id, new);
                        mark_dirty(new, NodeMask::ALL, &mut nodes_updated);
                    }
                }
                SetAttribute {
                    name,
                    value,
                    id,
                    ns,
                } => {
                    let node_id = self.element_to_node_id(id);
                    let node = self.tree.get_mut(node_id).unwrap();
                    if let NodeType::Element { attributes, .. } = &mut node.node_data.node_type {
                        if let BorrowedAttributeValue::None = &value {
                            attributes.remove(&OwnedAttributeDiscription {
                                name: name.to_string(),
                                namespace: ns.map(|s| s.to_string()),
                                volatile: false,
                            });
                            mark_dirty(
                                node_id,
                                NodeMask::new_with_attrs(AttributeMask::single(name)),
                                &mut nodes_updated,
                            );
                        } else {
                            attributes.insert(
                                OwnedAttributeDiscription {
                                    name: name.to_string(),
                                    namespace: ns.map(|s| s.to_string()),
                                    volatile: false,
                                },
                                OwnedAttributeValue::from(value),
                            );
                            mark_dirty(
                                node_id,
                                NodeMask::new_with_attrs(AttributeMask::single(name)),
                                &mut nodes_updated,
                            );
                        }
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
                NewEventListener { name, id } => {
                    let node_id = self.element_to_node_id(id);
                    let node = self.tree.get_mut(node_id).unwrap();
                    if let NodeType::Element { listeners, .. } = &mut node.node_data.node_type {
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
                        listeners.insert(name.to_string());
                    }
                }
                RemoveEventListener { id, name } => {
                    let node_id = self.element_to_node_id(id);
                    let node = self.tree.get_mut(node_id).unwrap();
                    if let NodeType::Element { listeners, .. } = &mut node.node_data.node_type {
                        listeners.remove(name);
                    }
                    self.nodes_listening.get_mut(name).unwrap().remove(&node_id);
                }
                Remove { id } => {
                    let node_id = self.element_to_node_id(id);
                    self.remove(node_id, &mut nodes_updated);
                }
                PushRoot { id } => {
                    let node_id = self.element_to_node_id(id);
                    self.stack.push(node_id);
                }
            }
        }

        let mut dirty_nodes = DirtyNodeStates::default();
        for (&n, mask) in &nodes_updated {
            // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
            if let Some(height) = self.tree.height(n) {
                for (m, p) in S::MASKS.iter().zip(S::PASSES.iter()) {
                    if mask.overlaps(m) {
                        dirty_nodes.insert(p.pass_id(), n, height);
                    }
                }
            }
        }

        (dirty_nodes, nodes_updated)
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state_single_threaded(
        &mut self,
        nodes_updated: DirtyNodeStates,
        ctx: SendAnyMap,
    ) -> FxDashSet<RealNodeId> {
        S::update_single_threaded(nodes_updated, &mut self.tree, ctx)
    }

    /// Find all nodes that are listening for an event, sorted by there height in the dom progressing starting at the bottom and progressing up.
    /// This can be useful to avoid creating duplicate events.
    pub fn get_listening_sorted(&self, event: &str) -> Vec<&Node<S, V>> {
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

    fn clone_node(
        &mut self,
        node_id: NodeId,
        nodes_updated: &mut FxHashMap<RealNodeId, NodeMask>,
    ) -> RealNodeId {
        let node = self.tree.get(node_id).unwrap();
        let new_node = node.clone();
        let new_id = self.create_node(new_node);
        mark_dirty(new_id, NodeMask::ALL, nodes_updated);
        let self_ptr = self as *mut Self;
        for child in self.tree.children_ids(node_id).unwrap() {
            unsafe {
                // this is safe because no node has itself as a child
                let self_mut = &mut *self_ptr;
                let child_id = self_mut.clone_node(*child, nodes_updated);
                self_mut.add_child(new_id, child_id);
            }
        }
        new_id
    }

    fn remove(&mut self, node_id: NodeId, nodes_updated: &mut FxHashMap<RealNodeId, NodeMask>) {
        let node = self.tree.get(node_id).unwrap();
        if let NodeType::Element { listeners, .. } = &node.node_data.node_type {
            for name in listeners.iter() {
                self.nodes_listening.get_mut(name).unwrap().remove(&node_id);
            }
        }
        if let Some(children) = self.tree.children_ids(node_id) {
            let children = children.to_vec();
            for child in children {
                self.remove(child, nodes_updated);
            }
        }
        self.tree.remove(node_id);
        mark_dirty(node_id, NodeMask::ALL, nodes_updated);
    }
}

impl<S: State<V> + Sync, V: FromAnyValue> RealDom<S, V>
where
    Tree<Node<S, V>>: Sync + Send,
{
    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    /// This will resolve the state in parallel
    pub fn update_state(
        &mut self,
        nodes_updated: DirtyNodeStates,
        ctx: SendAnyMap,
    ) -> FxDashSet<RealNodeId> {
        S::update(nodes_updated, &mut self.tree, ctx)
    }
}

impl<S: State<V>, V: FromAnyValue> Deref for RealDom<S, V> {
    type Target = Tree<Node<S, V>>;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl<S: State<V>, V: FromAnyValue> DerefMut for RealDom<S, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl<S: State<V>, V: FromAnyValue> Index<ElementId> for RealDom<S, V> {
    type Output = Node<S, V>;

    fn index(&self, id: ElementId) -> &Self::Output {
        self.tree.get(self.element_to_node_id(id)).unwrap()
    }
}

impl<S: State<V>, V: FromAnyValue> Index<RealNodeId> for RealDom<S, V> {
    type Output = Node<S, V>;

    fn index(&self, idx: RealNodeId) -> &Self::Output {
        self.tree.get(idx).unwrap()
    }
}

impl<S: State<V>, V: FromAnyValue> IndexMut<ElementId> for RealDom<S, V> {
    fn index_mut(&mut self, id: ElementId) -> &mut Self::Output {
        self.tree.get_mut(self.element_to_node_id(id)).unwrap()
    }
}

impl<S: State<V>, V: FromAnyValue> IndexMut<RealNodeId> for RealDom<S, V> {
    fn index_mut(&mut self, idx: RealNodeId) -> &mut Self::Output {
        self.tree.get_mut(idx).unwrap()
    }
}

use anymap::AnyMap;
use fxhash::{FxHashMap, FxHashSet};
use std::ops::{Index, IndexMut};

use dioxus_core::{ElementId, Mutations, VNode, VirtualDom};

use crate::node_ref::{AttributeMask, NodeMask};
use crate::state::State;
use crate::traversable::Traversable;

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [crate::state::ParentDepState], [crate::state::NodeDepState], or [crate::state::ChildDepState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
#[derive(Debug)]
pub struct RealDom<S: State> {
    root: usize,
    nodes: Vec<Option<Node<S>>>,
    nodes_listening: FxHashMap<&'static str, FxHashSet<ElementId>>,
    node_stack: smallvec::SmallVec<[usize; 10]>,
}

impl<S: State> Default for RealDom<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: State> RealDom<S> {
    pub fn new() -> RealDom<S> {
        RealDom {
            root: 0,
            nodes: {
                let v = vec![Some(Node::new(
                    0,
                    NodeType::Element {
                        tag: "Root".to_string(),
                        namespace: Some("Root"),
                        children: Vec::new(),
                    },
                ))];
                v
            },
            nodes_listening: FxHashMap::default(),
            node_stack: smallvec::SmallVec::new(),
        }
    }

    /// Updates the dom, up and down state and return a set of nodes that were updated pass this to update_state.
    pub fn apply_mutations(&mut self, mutations_vec: Vec<Mutations>) -> Vec<(ElementId, NodeMask)> {
        let mut nodes_updated = Vec::new();
        for mutations in mutations_vec {
            for e in mutations.edits {
                use dioxus_core::DomEdit::*;
                match e {
                    PushRoot { root } => self.node_stack.push(root as usize),
                    AppendChildren { many } => {
                        let target = if self.node_stack.len() > many as usize {
                            *self
                                .node_stack
                                .get(self.node_stack.len() - (many as usize + 1))
                                .unwrap()
                        } else {
                            0
                        };
                        let drained: Vec<_> = self
                            .node_stack
                            .drain(self.node_stack.len() - many as usize..)
                            .collect();
                        for ns in drained {
                            let id = ElementId(ns);
                            self.link_child(id, ElementId(target)).unwrap();
                            nodes_updated.push((id, NodeMask::ALL));
                        }
                    }
                    ReplaceWith { root, m } => {
                        let root = self.remove(ElementId(root as usize)).unwrap();
                        let target = root.parent.unwrap().0;
                        let drained: Vec<_> = self.node_stack.drain(0..m as usize).collect();
                        for ns in drained {
                            let id = ElementId(ns);
                            nodes_updated.push((id, NodeMask::ALL));
                            self.link_child(id, ElementId(target)).unwrap();
                        }
                    }
                    InsertAfter { root, n } => {
                        let target = self[ElementId(root as usize)].parent.unwrap().0;
                        let drained: Vec<_> = self.node_stack.drain(0..n as usize).collect();
                        for ns in drained {
                            let id = ElementId(ns);
                            nodes_updated.push((id, NodeMask::ALL));
                            self.link_child(id, ElementId(target)).unwrap();
                        }
                    }
                    InsertBefore { root, n } => {
                        let target = self[ElementId(root as usize)].parent.unwrap().0;
                        let drained: Vec<_> = self.node_stack.drain(0..n as usize).collect();
                        for ns in drained {
                            let id = ElementId(ns);
                            nodes_updated.push((id, NodeMask::ALL));
                            self.link_child(id, ElementId(target)).unwrap();
                        }
                    }
                    Remove { root } => {
                        if let Some(parent) = self[ElementId(root as usize)].parent {
                            nodes_updated.push((parent, NodeMask::NONE));
                        }
                        self.remove(ElementId(root as usize)).unwrap();
                    }
                    CreateTextNode { root, text } => {
                        let n = Node::new(
                            root,
                            NodeType::Text {
                                text: text.to_string(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(root as usize)
                    }
                    CreateElement { root, tag } => {
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: None,
                                children: Vec::new(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(root as usize)
                    }
                    CreateElementNs { root, tag, ns } => {
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: Some(ns),
                                children: Vec::new(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(root as usize)
                    }
                    CreatePlaceholder { root } => {
                        let n = Node::new(root, NodeType::Placeholder);
                        self.insert(n);
                        self.node_stack.push(root as usize)
                    }

                    NewEventListener {
                        event_name,
                        scope: _,
                        root,
                    } => {
                        let id = ElementId(root as usize);
                        nodes_updated.push((id, NodeMask::new().with_listeners()));
                        if let Some(v) = self.nodes_listening.get_mut(event_name) {
                            v.insert(id);
                        } else {
                            let mut hs = FxHashSet::default();
                            hs.insert(id);
                            self.nodes_listening.insert(event_name, hs);
                        }
                    }
                    RemoveEventListener { root, event } => {
                        let id = ElementId(root as usize);
                        nodes_updated.push((id, NodeMask::new().with_listeners()));
                        let v = self.nodes_listening.get_mut(event).unwrap();
                        v.remove(&id);
                    }
                    SetText {
                        root,
                        text: new_text,
                    } => {
                        let id = ElementId(root as usize);
                        let target = &mut self[id];
                        nodes_updated.push((id, NodeMask::new().with_text()));
                        match &mut target.node_type {
                            NodeType::Text { text } => {
                                *text = new_text.to_string();
                            }
                            _ => unreachable!(),
                        }
                    }
                    SetAttribute { root, field, .. } => {
                        let id = ElementId(root as usize);
                        nodes_updated
                            .push((id, NodeMask::new_with_attrs(AttributeMask::single(field))));
                    }
                    RemoveAttribute {
                        root, name: field, ..
                    } => {
                        let id = ElementId(root as usize);
                        nodes_updated
                            .push((id, NodeMask::new_with_attrs(AttributeMask::single(field))));
                    }
                    PopRoot {} => {
                        self.node_stack.pop();
                    }
                }
            }
        }

        nodes_updated
    }

    pub fn update_state(
        &mut self,
        vdom: &VirtualDom,
        nodes_updated: Vec<(ElementId, NodeMask)>,
        ctx: AnyMap,
    ) -> FxHashSet<ElementId> {
        S::update(
            &nodes_updated,
            &mut self.map(|n| &n.state, |n| &mut n.state),
            vdom,
            &ctx,
        )
    }

    fn link_child(&mut self, child_id: ElementId, parent_id: ElementId) -> Option<()> {
        debug_assert_ne!(child_id, parent_id);
        let parent = &mut self[parent_id];
        parent.add_child(child_id);
        let parent_height = parent.height + 1;
        self[child_id].set_parent(parent_id);
        self.increase_height(child_id, parent_height);
        Some(())
    }

    fn increase_height(&mut self, id: ElementId, amount: u16) {
        let n = &mut self[id];
        n.height += amount;
        if let NodeType::Element { children, .. } = &n.node_type {
            for c in children.clone() {
                self.increase_height(c, amount);
            }
        }
    }

    // remove a node and it's children from the dom.
    fn remove(&mut self, id: ElementId) -> Option<Node<S>> {
        // We do not need to remove the node from the parent's children list for children.
        fn inner<S: State>(dom: &mut RealDom<S>, id: ElementId) -> Option<Node<S>> {
            let mut node = dom.nodes[id.0].take()?;
            if let NodeType::Element { children, .. } = &mut node.node_type {
                for c in children {
                    inner(dom, *c)?;
                }
            }
            Some(node)
        }
        let mut node = self.nodes[id.0].take()?;
        if let Some(parent) = node.parent {
            let parent = &mut self[parent];
            parent.remove_child(id);
        }
        if let NodeType::Element { children, .. } = &mut node.node_type {
            for c in children {
                inner(self, *c)?;
            }
        }
        Some(node)
    }

    fn insert(&mut self, node: Node<S>) {
        let current_len = self.nodes.len();
        let id = node.id.0;
        if current_len - 1 < node.id.0 {
            // self.nodes.reserve(1 + id - current_len);
            self.nodes.extend((0..1 + id - current_len).map(|_| None));
        }
        self.nodes[id] = Some(node);
    }

    pub fn get_listening_sorted(&self, event: &'static str) -> Vec<&Node<S>> {
        if let Some(nodes) = self.nodes_listening.get(event) {
            let mut listening: Vec<_> = nodes.iter().map(|id| &self[*id]).collect();
            listening.sort_by(|n1, n2| (n1.height).cmp(&n2.height).reverse());
            listening
        } else {
            Vec::new()
        }
    }

    /// Check if the dom contains a node and its children.
    pub fn contains_node(&self, node: &VNode) -> bool {
        match node {
            VNode::Component(_) => {
                todo!()
            }
            VNode::Element(e) => {
                if let Some(id) = e.id.get() {
                    let dom_node = &self[id];
                    match &dom_node.node_type {
                        NodeType::Element {
                            tag,
                            namespace,
                            children,
                        } => {
                            tag == e.tag
                                && namespace == &e.namespace
                                && children.iter().copied().collect::<FxHashSet<_>>()
                                    == e.children
                                        .iter()
                                        .map(|c| c.mounted_id())
                                        .collect::<FxHashSet<_>>()
                                && e.children.iter().all(|c| {
                                    self.contains_node(c)
                                        && self[c.mounted_id()].parent == e.id.get()
                                })
                        }
                        _ => false,
                    }
                } else {
                    true
                }
            }
            VNode::Fragment(f) => f.children.iter().all(|c| self.contains_node(c)),
            VNode::Placeholder(_) => true,
            VNode::Text(t) => {
                if let Some(id) = t.id.get() {
                    let dom_node = &self[id];
                    match &dom_node.node_type {
                        NodeType::Text { text } => t.text == text,
                        _ => false,
                    }
                } else {
                    true
                }
            }
        }
    }

    /// Return the number of nodes in the dom.
    pub fn size(&self) -> usize {
        // The dom has a root node, ignore it.
        self.nodes.iter().filter(|n| n.is_some()).count() - 1
    }

    /// Returns the id of the root node.
    pub fn root_id(&self) -> usize {
        self.root
    }

    /// Call a function for each node in the dom, depth first.
    pub fn traverse_depth_first(&self, mut f: impl FnMut(&Node<S>)) {
        fn inner<S: State>(dom: &RealDom<S>, id: ElementId, f: &mut impl FnMut(&Node<S>)) {
            let node = &dom[id];
            f(node);
            if let NodeType::Element { children, .. } = &node.node_type {
                for c in children {
                    inner(dom, *c, f);
                }
            }
        }
        if let NodeType::Element { children, .. } = &self[ElementId(self.root)].node_type {
            for c in children {
                inner(self, *c, &mut f);
            }
        }
    }

    /// Call a function for each node in the dom, depth first.
    pub fn traverse_depth_first_mut(&mut self, mut f: impl FnMut(&mut Node<S>)) {
        fn inner<S: State>(dom: &mut RealDom<S>, id: ElementId, f: &mut impl FnMut(&mut Node<S>)) {
            let node = &mut dom[id];
            f(node);
            if let NodeType::Element { children, .. } = &mut node.node_type {
                for c in children.clone() {
                    inner(dom, c, f);
                }
            }
        }
        let root = self.root;
        if let NodeType::Element { children, .. } = &mut self[ElementId(root)].node_type {
            for c in children.clone() {
                inner(self, c, &mut f);
            }
        }
    }
}

impl<S: State> Index<ElementId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: ElementId) -> &Self::Output {
        self.get(idx).unwrap()
    }
}

impl<S: State> IndexMut<ElementId> for RealDom<S> {
    fn index_mut(&mut self, idx: ElementId) -> &mut Self::Output {
        self.get_mut(idx).unwrap()
    }
}

/// The node is stored client side and stores only basic data about the node.
#[derive(Debug, Clone)]
pub struct Node<S: State> {
    /// The id of the node this node was created from.
    pub id: ElementId,
    /// The parent id of the node.
    pub parent: Option<ElementId>,
    /// State of the node.
    pub state: S,
    /// Additional inforation specific to the node type
    pub node_type: NodeType,
    /// The number of parents before the root node. The root node has height 1.
    pub height: u16,
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Text {
        text: String,
    },
    Element {
        tag: String,
        namespace: Option<&'static str>,
        children: Vec<ElementId>,
    },
    Placeholder,
}

impl<S: State> Node<S> {
    fn new(id: u64, node_type: NodeType) -> Self {
        Node {
            id: ElementId(id as usize),
            parent: None,
            node_type,
            state: S::default(),
            height: 0,
        }
    }

    /// Returns a reference to the element that this node refrences.
    pub fn element<'b>(&self, vdom: &'b VirtualDom) -> &'b VNode<'b> {
        vdom.get_element(self.id).unwrap()
    }

    fn add_child(&mut self, child: ElementId) {
        if let NodeType::Element { children, .. } = &mut self.node_type {
            children.push(child);
        }
    }

    fn remove_child(&mut self, child: ElementId) {
        if let NodeType::Element { children, .. } = &mut self.node_type {
            children.retain(|c| c != &child);
        }
    }

    fn set_parent(&mut self, parent: ElementId) {
        self.parent = Some(parent);
    }
}

impl<T: State> Traversable for RealDom<T> {
    type Id = ElementId;
    type Node = Node<T>;

    fn height(&self, id: Self::Id) -> Option<u16> {
        Some(<Self as Traversable>::get(self, id)?.height)
    }

    fn get(&self, id: Self::Id) -> Option<&Self::Node> {
        self.nodes.get(id.0)?.as_ref()
    }

    fn get_mut(&mut self, id: Self::Id) -> Option<&mut Self::Node> {
        self.nodes.get_mut(id.0)?.as_mut()
    }

    fn children(&self, id: Self::Id) -> &[Self::Id] {
        if let Some(node) = <Self as Traversable>::get(self, id) {
            match &node.node_type {
                NodeType::Element { children, .. } => children,
                _ => &[],
            }
        } else {
            &[]
        }
    }

    fn parent(&self, id: Self::Id) -> Option<Self::Id> {
        <Self as Traversable>::get(self, id).and_then(|n| n.parent)
    }
}

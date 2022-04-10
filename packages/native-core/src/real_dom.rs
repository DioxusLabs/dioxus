use anymap::AnyMap;
use fxhash::{FxHashMap, FxHashSet};
use std::{
    collections::VecDeque,
    ops::{Index, IndexMut},
};

use dioxus_core::{ElementId, Mutations, VNode, VirtualDom};

use crate::real_dom_new_api::{State, NodeMask};

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [PushedDownState] and or [BubbledUpState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
#[derive(Debug)]
pub struct RealDom<S: State> {
    root: usize,
    nodes: Vec<Option<Node<S>>>,
    nodes_listening: FxHashMap<&'static str, FxHashSet<usize>>,
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
    pub fn apply_mutations(&mut self, mutations_vec: Vec<Mutations>) -> Vec<usize> {
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
                            self.link_child(ns, target).unwrap();
                            nodes_updated.push(ns);
                        }
                    }
                    ReplaceWith { root, m } => {
                        let root = self.remove(root as usize).unwrap();
                        let target = root.parent.unwrap().0;
                        let drained: Vec<_> = self.node_stack.drain(0..m as usize).collect();
                        for ns in drained {
                            nodes_updated.push(ns);
                            self.link_child(ns, target).unwrap();
                        }
                    }
                    InsertAfter { root, n } => {
                        let target = self[root as usize].parent.unwrap().0;
                        let drained: Vec<_> = self.node_stack.drain(0..n as usize).collect();
                        for ns in drained {
                            nodes_updated.push(ns);
                            self.link_child(ns, target).unwrap();
                        }
                    }
                    InsertBefore { root, n } => {
                        let target = self[root as usize].parent.unwrap().0;
                        let drained: Vec<_> = self.node_stack.drain(0..n as usize).collect();
                        for ns in drained {
                            nodes_updated.push(ns);
                            self.link_child(ns, target).unwrap();
                        }
                    }
                    Remove { root } => {
                        if let Some(parent) = self[root as usize].parent {
                            nodes_updated.push(parent.0);
                        }
                        self.remove(root as usize).unwrap();
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
                        if let Some(v) = self.nodes_listening.get_mut(event_name) {
                            v.insert(root as usize);
                        } else {
                            let mut hs = FxHashSet::default();
                            hs.insert(root as usize);
                            self.nodes_listening.insert(event_name, hs);
                        }
                    }
                    RemoveEventListener { root, event } => {
                        let v = self.nodes_listening.get_mut(event).unwrap();
                        v.remove(&(root as usize));
                    }
                    SetText {
                        root,
                        text: new_text,
                    } => {
                        let target = &mut self[root as usize];
                        nodes_updated.push(root as usize);
                        match &mut target.node_type {
                            NodeType::Text { text } => {
                                *text = new_text.to_string();
                            }
                            _ => unreachable!(),
                        }
                    }
                    SetAttribute { root, .. } => {
                        nodes_updated.push(root as usize);
                    }
                    RemoveAttribute { root, .. } => {
                        nodes_updated.push(root as usize);
                    }
                    PopRoot {} => {
                        self.node_stack.pop();
                    }
                }
            }
        }

        nodes_updated
    }

    /// Seperated from apply_mutations because Mutations require a mutable reference to the VirtualDom.
    pub fn update_state(
        &mut self,
        vdom: &VirtualDom,
        nodes_updated: Vec<usize>,
        ctx: AnyMap,
    ) -> Option<FxHashSet<usize>> {
        let mut to_rerender = FxHashSet::default();
        to_rerender.extend(nodes_updated.iter());
        let mut nodes_updated: Vec<_> = nodes_updated
            .into_iter()
            .map(|id| (id, self[id].height))
            .collect();
        // Sort nodes first by height, then if the height is the same id.
        nodes_updated.sort_by(|fst, snd| fst.1.cmp(&snd.1).then(fst.0.cmp(&snd.0)));
        nodes_updated.dedup();

        // bubble up state. To avoid calling reduce more times than nessisary start from the bottom and go up.
        let mut to_bubble: VecDeque<_> = nodes_updated.clone().into();
        while let Some((id, height)) = to_bubble.pop_back() {
            let node = &mut self[id as usize];
            let vnode = node.element(vdom);
            let node_type = &node.node_type;
            if let
                NodeType::Element { children, tag, namespace } =node.node_type{

                }
            let mask = NodeMask::new(attritutes, tag, namespace)
            // todo: reduce cloning state
            for id in node.state.child_dep_types(mask) {}
            if new != old {
                to_rerender.insert(id);
                if let Some(p) = parent {
                    let i = to_bubble.partition_point(|(other_id, h)| {
                        *h < height - 1 || (*h == height - 1 && *other_id < p.0)
                    });
                    // make sure the parent is not already queued
                    if i >= to_bubble.len() || to_bubble[i].0 != p.0 {
                        to_bubble.insert(i, (p.0, height - 1));
                    }
                }
                let node = &mut self[id as usize];
                node.up_state = new;
            }
        }

        // push down state. To avoid calling reduce more times than nessisary start from the top and go down.
        let mut to_push: VecDeque<_> = nodes_updated.clone().into();
        while let Some((id, height)) = to_push.pop_front() {
            let node = &self[id as usize];
            // todo: reduce cloning state
            let old = node.down_state.clone();
            let mut new = node.down_state.clone();
            let vnode = node.element(vdom);
            new.reduce(
                node.parent
                    .filter(|e| e.0 != 0)
                    .map(|e| &self[e].down_state),
                vnode,
                ds_ctx,
            );
            if new != old {
                to_rerender.insert(id);
                let node = &mut self[id as usize];
                if let NodeType::Element { children, .. } = &node.node_type {
                    for c in children {
                        let i = to_push.partition_point(|(other_id, h)| {
                            *h < height + 1 || (*h == height + 1 && *other_id < c.0)
                        });
                        if i >= to_push.len() || to_push[i].0 != c.0 {
                            to_push.insert(i, (c.0, height + 1));
                        }
                    }
                }
                node.down_state = new;
            }
        }

        Some(to_rerender)
    }

    fn link_child(&mut self, child_id: usize, parent_id: usize) -> Option<()> {
        debug_assert_ne!(child_id, parent_id);
        let parent = &mut self[parent_id];
        parent.add_child(ElementId(child_id));
        let parent_height = parent.height + 1;
        self[child_id].set_parent(ElementId(parent_id));
        self.increase_height(child_id, parent_height);
        Some(())
    }

    fn increase_height(&mut self, id: usize, amount: u16) {
        let n = &mut self[id];
        n.height += amount;
        if let NodeType::Element { children, .. } = &n.node_type {
            for c in children.clone() {
                self.increase_height(c.0, amount);
            }
        }
    }

    // remove a node and it's children from the dom.
    fn remove(&mut self, id: usize) -> Option<Node<S>> {
        // We do not need to remove the node from the parent's children list for children.
        fn inner<S: State>(dom: &mut RealDom<S>, id: usize) -> Option<Node<S>> {
            let mut node = dom.nodes[id as usize].take()?;
            if let NodeType::Element { children, .. } = &mut node.node_type {
                for c in children {
                    inner(dom, c.0)?;
                }
            }
            Some(node)
        }
        let mut node = self.nodes[id as usize].take()?;
        if let Some(parent) = node.parent {
            let parent = &mut self[parent];
            parent.remove_child(ElementId(id));
        }
        if let NodeType::Element { children, .. } = &mut node.node_type {
            for c in children {
                inner(self, c.0)?;
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

    pub fn get(&self, id: usize) -> Option<&Node<S>> {
        self.nodes.get(id)?.as_ref()
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Node<S>> {
        self.nodes.get_mut(id)?.as_mut()
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
        if let NodeType::Element { children, .. } = &self[self.root].node_type {
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
        if let NodeType::Element { children, .. } = &mut self[root].node_type {
            for c in children.clone() {
                inner(self, c, &mut f);
            }
        }
    }
}

impl<S: State> Index<usize> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: usize) -> &Self::Output {
        self.get(idx).expect("Node does not exist")
    }
}

impl<S: State> Index<ElementId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: ElementId) -> &Self::Output {
        &self[idx.0]
    }
}

impl<S: State> IndexMut<usize> for RealDom<S> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        self.get_mut(idx).expect("Node does not exist")
    }
}
impl<S: State> IndexMut<ElementId> for RealDom<S> {
    fn index_mut(&mut self, idx: ElementId) -> &mut Self::Output {
        &mut self[idx.0]
    }
}

/// The node is stored client side and stores only basic data about the node. For more complete information about the node see [`domNode::element`].
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

/// This state that is passed down to children. For example text properties (`<b>` `<i>` `<u>`) would be passed to children.
/// Called when the current node's node properties are modified or a parrent's [PushedDownState] is modified.
/// Called at most once per update.
pub trait PushedDownState: Default + PartialEq + Clone {
    /// The context is passed to the [PushedDownState::reduce] when it is pushed down.
    /// This is sometimes nessisary for lifetime purposes.
    type Ctx;
    fn reduce(&mut self, parent: Option<&Self>, vnode: &VNode, ctx: &mut Self::Ctx);
}
impl PushedDownState for () {
    type Ctx = ();
    fn reduce(&mut self, _parent: Option<&Self>, _vnode: &VNode, _ctx: &mut Self::Ctx) {}
}

/// This state is derived from children. For example a node's size could be derived from the size of children.
/// Called when the current node's node properties are modified, a child's [BubbledUpState] is modified or a child is removed.
/// Called at most once per update.
pub trait BubbledUpState: Default + PartialEq + Clone {
    /// The context is passed to the [BubbledUpState::reduce] when it is bubbled up.
    /// This is sometimes nessisary for lifetime purposes.
    type Ctx;
    fn reduce<'a, I>(&mut self, children: I, vnode: &VNode, ctx: &mut Self::Ctx)
    where
        I: Iterator<Item = &'a Self>,
        Self: 'a;
}
impl BubbledUpState for () {
    type Ctx = ();
    fn reduce<'a, I>(&mut self, _children: I, _vnode: &VNode, _ctx: &mut Self::Ctx)
    where
        I: Iterator<Item = &'a Self>,
        Self: 'a,
    {
    }
}

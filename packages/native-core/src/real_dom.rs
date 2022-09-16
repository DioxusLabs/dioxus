use anymap::AnyMap;
use fxhash::{FxHashMap, FxHashSet};
use std::ops::{Index, IndexMut};

use dioxus_core::{
    AttributeDiscription, ElementId, GlobalNodeId, Mutations, OwnedAttributeValue,
    RendererTemplateId, TemplateNodeId, VNode, VirtualDom, JS_MAX_INT,
};

use crate::node_ref::{AttributeMask, NodeMask};
use crate::state::State;
use crate::template::{NativeTemplate, TemplateRefOrNode};
use crate::traversable::Traversable;

pub(crate) type TemplateMapping<S> = FxHashMap<RendererTemplateId, NativeTemplate<S>>;

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [crate::state::ParentDepState], [crate::state::NodeDepState], or [crate::state::ChildDepState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
#[derive(Debug)]
pub struct RealDom<S: State> {
    root: usize,
    nodes: Vec<Option<Box<TemplateRefOrNode<S>>>>,
    nodes_listening: FxHashMap<&'static str, FxHashSet<GlobalNodeId>>,
    templates: TemplateMapping<S>,
    template_stack: smallvec::SmallVec<[ElementId; 5]>,
    template_in_progress: Option<RendererTemplateId>,
    node_stack: smallvec::SmallVec<[GlobalNodeId; 10]>,
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
                let v = vec![Some(Box::new(TemplateRefOrNode::Node(Node::new(
                    0,
                    NodeType::Element {
                        tag: "Root".to_string(),
                        namespace: Some("Root"),
                        attributes: FxHashMap::default(),
                        listeners: Vec::new(),
                        children: Vec::new(),
                    },
                ))))];
                v
            },
            nodes_listening: FxHashMap::default(),
            node_stack: smallvec::SmallVec::new(),
            templates: FxHashMap::default(),
            template_stack: smallvec::SmallVec::new(),
            template_in_progress: None,
        }
    }

    /// Updates the dom, up and down state and return a set of nodes that were updated pass this to update_state.
    pub fn apply_mutations(
        &mut self,
        mutations_vec: Vec<Mutations>,
    ) -> Vec<(GlobalNodeId, NodeMask)> {
        let mut nodes_updated = Vec::new();
        for mutations in mutations_vec {
            for e in mutations.edits {
                use dioxus_core::DomEdit::*;
                match e {
                    PushRoot { root } => self.node_stack.push(self.decode_id(root)),
                    AppendChildren { many } => {
                        let target = if self.node_stack.len() > many as usize {
                            *self
                                .node_stack
                                .get(self.node_stack.len() - (many as usize + 1))
                                .unwrap()
                        } else {
                            GlobalNodeId::VNodeId(ElementId(0))
                        };
                        let drained: Vec<_> = self
                            .node_stack
                            .drain(self.node_stack.len() - many as usize..)
                            .collect();
                        for id in drained {
                            self.link_child(id, target).unwrap();
                            nodes_updated.push((id, NodeMask::ALL));
                        }
                    }
                    ReplaceWith { root, m } => {
                        let id = self.decode_id(root);
                        let root = self.remove(id).unwrap();
                        let target = root.parent().unwrap();
                        let drained: Vec<_> = self.node_stack.drain(0..m as usize).collect();
                        for id in drained {
                            nodes_updated.push((id, NodeMask::ALL));
                            self.link_child(id, target).unwrap();
                        }
                    }
                    InsertAfter { root, n } => {
                        let target = self[root as usize].node_data.parent.unwrap();
                        let drained: Vec<_> = self.node_stack.drain(0..n as usize).collect();
                        for id in drained {
                            nodes_updated.push((id, NodeMask::ALL));
                            self.link_child(id, target).unwrap();
                        }
                    }
                    InsertBefore { root, n } => {
                        let target = self[root as usize].node_data.parent.unwrap();
                        let drained: Vec<_> = self.node_stack.drain(0..n as usize).collect();
                        for id in drained {
                            nodes_updated.push((id, NodeMask::ALL));
                            self.link_child(id, target).unwrap();
                        }
                    }
                    Remove { root } => {
                        if let Some(parent) = self[root as usize].node_data.parent {
                            nodes_updated.push((parent, NodeMask::NONE));
                        }
                        let id = self.decode_id(root);
                        self.remove(id).unwrap();
                    }
                    CreateTextNode { root, text } => {
                        let n = Node::new(
                            root,
                            NodeType::Text {
                                text: text.to_string(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(self.decode_id(root))
                    }
                    CreateTextNodeTemplate {
                        root,
                        text,
                        locally_static: _,
                    } => {
                        let n = Node::new(
                            root,
                            NodeType::Text {
                                text: text.to_string(),
                            },
                        );
                        self.current_template_mut().unwrap().insert(n);
                        self.node_stack.push(self.decode_id(root))
                    }
                    CreateElement { root, tag } => {
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: None,
                                attributes: FxHashMap::default(),
                                listeners: Vec::new(),
                                children: Vec::new(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(self.decode_id(root))
                    }
                    CreateElementTemplate {
                        root,
                        tag,
                        locally_static: _,
                        fully_static: _,
                    } => {
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: None,
                                attributes: FxHashMap::default(),
                                listeners: Vec::new(),
                                children: Vec::new(),
                            },
                        );
                        self.current_template_mut().unwrap().insert(n);
                        self.node_stack.push(self.decode_id(root))
                    }
                    CreateElementNs { root, tag, ns } => {
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: Some(ns),
                                attributes: FxHashMap::default(),
                                listeners: Vec::new(),
                                children: Vec::new(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(self.decode_id(root))
                    }
                    CreateElementNsTemplate {
                        root,
                        tag,
                        ns,
                        locally_static: _,
                        fully_static: _,
                    } => {
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: Some(ns),
                                attributes: FxHashMap::default(),
                                listeners: Vec::new(),
                                children: Vec::new(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(self.decode_id(root))
                    }
                    CreatePlaceholder { root } => {
                        let n = Node::new(root, NodeType::Placeholder);
                        self.insert(n);
                        self.node_stack.push(self.decode_id(root))
                    }
                    CreatePlaceholderTemplate { root } => {
                        let n = Node::new(root, NodeType::Placeholder);
                        self.current_template_mut().unwrap().insert(n);
                        self.node_stack.push(self.decode_id(root))
                    }
                    NewEventListener {
                        event_name,
                        scope: _,
                        root,
                    } => {
                        let id = self.decode_id(root);
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
                        let id = self.decode_id(root);
                        nodes_updated.push((id, NodeMask::new().with_listeners()));
                        let v = self.nodes_listening.get_mut(event).unwrap();
                        v.remove(&id);
                    }
                    SetText {
                        root,
                        text: new_text,
                    } => {
                        let id = self.decode_id(root);
                        let target = &mut self[id];
                        nodes_updated.push((id, NodeMask::new().with_text()));
                        match &mut target.node_data.node_type {
                            NodeType::Text { text } => {
                                *text = new_text.to_string();
                            }
                            _ => unreachable!(),
                        }
                    }
                    SetAttribute {
                        root,
                        field,
                        ns,
                        value,
                    } => {
                        let id = self.decode_id(root);
                        if let NodeType::Element { attributes, .. } =
                            &mut self[id].node_data.node_type
                        {
                            attributes.insert(
                                OwnedAttributeDiscription {
                                    name: field.to_owned(),
                                    namespace: ns.map(|a| a.to_owned()),
                                    volatile: false,
                                },
                                value.into(),
                            );
                        } else {
                            panic!("tried to call set attribute on a non element");
                        }
                        nodes_updated
                            .push((id, NodeMask::new_with_attrs(AttributeMask::single(field))));
                    }
                    RemoveAttribute {
                        root, name: field, ..
                    } => {
                        let id = self.decode_id(root);
                        nodes_updated
                            .push((id, NodeMask::new_with_attrs(AttributeMask::single(field))));
                    }
                    PopRoot {} => {
                        self.node_stack.pop();
                    }
                    CreateTemplateRef { id, template_id } => {
                        let template_id = RendererTemplateId(template_id as usize);
                        let template = self.templates.get(&template_id).unwrap();
                        let nodes = template.nodes.clone();
                        let id = ElementId(id as usize);
                        let template_ref = TemplateRefOrNode::Ref {
                            nodes,
                            parent: None,
                        };
                        self.nodes[id.0] = Some(Box::new(template_ref));
                        self.node_stack.push(dioxus_core::GlobalNodeId::VNodeId(id));
                    }
                    CreateTemplate { id } => {
                        let id = RendererTemplateId(id as usize);
                        self.templates.insert(id, NativeTemplate::default());
                        self.template_in_progress = Some(id);
                    }
                    FinishTemplate { len } => {
                        let len = len as usize;
                        let roots = self
                            .node_stack
                            .drain((self.node_stack.len() - len)..)
                            .map(|id| {
                                if let GlobalNodeId::TemplateId {
                                    template_node_id, ..
                                } = id
                                {
                                    template_node_id.0
                                } else {
                                    panic!("tried to add a non-template node to a template")
                                }
                            })
                            .collect();
                        let current_template = self.current_template_mut();
                        current_template.unwrap().roots = roots;
                        self.template_in_progress = None;
                    }
                    EnterTemplateRef { root } => self.template_stack.push(ElementId(root as usize)),
                    ExitTemplateRef {} => {
                        self.template_stack.pop();
                    }
                }
            }
        }

        debug_assert!(self.template_stack.is_empty());
        debug_assert_eq!(self.template_in_progress, None);

        nodes_updated
    }

    fn current_template_mut(&mut self) -> Option<&mut NativeTemplate<S>> {
        self.templates.get_mut(self.template_in_progress.as_ref()?)
    }

    fn current_template(&self) -> Option<&NativeTemplate<S>> {
        self.templates.get(self.template_in_progress.as_ref()?)
    }

    pub fn update_state(
        &mut self,
        nodes_updated: Vec<(GlobalNodeId, NodeMask)>,
        ctx: AnyMap,
    ) -> FxHashSet<GlobalNodeId> {
        let (mut state_tree, node_tree) = self.split();
        S::update(&nodes_updated, &mut state_tree, &node_tree, &ctx)
    }

    fn link_child(&mut self, child_id: GlobalNodeId, parent_id: GlobalNodeId) -> Option<()> {
        debug_assert_ne!(child_id, parent_id);
        let parent = &mut self[parent_id];
        parent.add_child(child_id);
        let parent_height = parent.node_data.height + 1;
        self[child_id].set_parent(parent_id);
        if let GlobalNodeId::VNodeId(child_id) = child_id {
            self.increase_height(child_id, parent_height);
        }
        Some(())
    }

    fn increase_height(&mut self, id: ElementId, amount: u16) {
        let n = &mut self[GlobalNodeId::VNodeId(id)];
        n.node_data.height += amount;
        if let NodeType::Element { children, .. } = &n.node_data.node_type {
            for c in children.clone() {
                if let GlobalNodeId::VNodeId(c) = c {
                    self.increase_height(c, amount);
                }
            }
        }
    }

    // remove a node and it's children from the dom.
    fn remove(&mut self, id: GlobalNodeId) -> Option<TemplateRefOrNode<S>> {
        // We do not need to remove the node from the parent's children list for children.
        fn inner<S: State>(dom: &mut RealDom<S>, id: GlobalNodeId) -> Option<TemplateRefOrNode<S>> {
            let mut either = match id {
                GlobalNodeId::VNodeId(id) => *dom.nodes[id.0].take()?,
                GlobalNodeId::TemplateId {
                    template_ref_id,
                    template_node_id,
                } => {
                    let template_ref = &mut dom.nodes[template_ref_id.0].as_mut().unwrap();
                    if let TemplateRefOrNode::Ref { nodes, .. } = template_ref.as_mut() {
                        TemplateRefOrNode::Node(*nodes[template_node_id.0].take().unwrap())
                    } else {
                        unreachable!()
                    }
                }
            };
            match &mut either {
                TemplateRefOrNode::Node(node) => {
                    if let NodeType::Element { children, .. } = &mut node.node_data.node_type {
                        for c in children {
                            inner(dom, *c);
                        }
                    }
                    Some(either)
                }
                TemplateRefOrNode::Ref { .. } => Some(either),
            }
        }
        let mut node = match id {
            GlobalNodeId::VNodeId(id) => *self.nodes[id.0].take()?,
            GlobalNodeId::TemplateId {
                template_ref_id,
                template_node_id,
            } => {
                let template_ref = &mut self.nodes[template_ref_id.0].as_mut().unwrap();
                if let TemplateRefOrNode::Ref { nodes, .. } = template_ref.as_mut() {
                    TemplateRefOrNode::Node(*nodes[template_node_id.0].take().unwrap())
                } else {
                    unreachable!()
                }
            }
        };
        if let Some(parent) = node.parent() {
            let parent = &mut self[parent];
            parent.remove_child(id);
        }
        match &mut node {
            TemplateRefOrNode::Ref { .. } => {}
            TemplateRefOrNode::Node(node) => {
                if let NodeType::Element { children, .. } = &mut node.node_data.node_type {
                    for c in children {
                        inner(self, *c)?;
                    }
                }
            }
        }
        Some(node)
    }

    fn insert(&mut self, node: Node<S>) {
        let current_len = self.nodes.len();
        let id = node.node_data.id.0;
        if current_len - 1 < node.node_data.id.0 {
            // self.nodes.reserve(1 + id - current_len);
            self.nodes.extend((0..1 + id - current_len).map(|_| None));
        }
        self.nodes[id] = Some(Box::new(TemplateRefOrNode::Node(node)));
    }

    pub fn get_listening_sorted(&self, event: &'static str) -> Vec<&Node<S>> {
        if let Some(nodes) = self.nodes_listening.get(event) {
            let mut listening: Vec<_> = nodes.iter().map(|id| &self[*id]).collect();
            listening.sort_by(|n1, n2| (n1.node_data.height).cmp(&n2.node_data.height).reverse());
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
                    let dom_node = &self[GlobalNodeId::VNodeId(id)];
                    match &dom_node.node_data.node_type {
                        NodeType::Element {
                            tag,
                            namespace,
                            children,
                            attributes,
                            listeners,
                        } => {
                            tag == e.tag
                                && namespace == &e.namespace
                                && children
                                    .iter()
                                    .zip(
                                        e.children
                                            .iter()
                                            .map(|c| GlobalNodeId::VNodeId(c.mounted_id())),
                                    )
                                    .all(|(c1, c2)| *c1 == c2)
                                && e.children.iter().all(|c| {
                                    self.contains_node(c)
                                        && self[GlobalNodeId::VNodeId(c.mounted_id())]
                                            .node_data
                                            .parent
                                            == e.id.get().map(|id| GlobalNodeId::VNodeId(id))
                                })
                                && attributes
                                    .iter()
                                    .zip(e.attributes.iter())
                                    .all(|((disc, val), b)| *disc == b.attribute && *val == b.value)
                                && listeners
                                    .iter()
                                    .zip(e.listeners.iter())
                                    .all(|(a, b)| *a == b.event)
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
                    let dom_node = &self[GlobalNodeId::VNodeId(id)];
                    match &dom_node.node_data.node_type {
                        NodeType::Text { text } => t.text == text,
                        _ => false,
                    }
                } else {
                    true
                }
            }
            VNode::TemplateRef(_) => todo!(),
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
        fn inner<S: State>(dom: &RealDom<S>, id: GlobalNodeId, f: &mut impl FnMut(&Node<S>)) {
            let node = &dom[id];
            f(node);
            if let NodeType::Element { children, .. } = &node.node_data.node_type {
                for c in children {
                    inner(dom, *c, f);
                }
            }
        }
        if let NodeType::Element { children, .. } = &self
            [GlobalNodeId::VNodeId(ElementId(self.root))]
        .node_data
        .node_type
        {
            for c in children {
                inner(self, *c, &mut f);
            }
        }
    }

    /// Call a function for each node in the dom, depth first.
    pub fn traverse_depth_first_mut(&mut self, mut f: impl FnMut(&mut Node<S>)) {
        fn inner<S: State>(
            dom: &mut RealDom<S>,
            id: GlobalNodeId,
            f: &mut impl FnMut(&mut Node<S>),
        ) {
            let node = &mut dom[id];
            f(node);
            if let NodeType::Element { children, .. } = &mut node.node_data.node_type {
                for c in children.clone() {
                    inner(dom, c, f);
                }
            }
        }
        let root = self.root;
        if let NodeType::Element { children, .. } = &mut self
            [GlobalNodeId::VNodeId(ElementId(root))]
        .node_data
        .node_type
        {
            for c in children.clone() {
                inner(self, c, &mut f);
            }
        }
    }

    pub fn decode_id(&self, id: impl Into<u64>) -> GlobalNodeId {
        let id = id.into();
        if id > JS_MAX_INT / 2 {
            if self.current_template().is_some() {
                GlobalNodeId::TemplateId {
                    template_ref_id: ElementId(0),
                    template_node_id: TemplateNodeId(id as usize),
                }
            } else {
                let template_ref_id = *self.template_stack.last().unwrap();
                let template_node_id = TemplateNodeId((id - (JS_MAX_INT / 2)) as usize);
                GlobalNodeId::TemplateId {
                    template_ref_id,
                    template_node_id,
                }
            }
        } else {
            GlobalNodeId::VNodeId(ElementId(id as usize))
        }
    }

    pub fn split<'a>(
        &'a mut self,
    ) -> (
        impl Traversable<Id = GlobalNodeId, Node = S> + 'a,
        impl Traversable<Id = GlobalNodeId, Node = NodeData> + 'a,
    ) {
        let raw = self as *mut Self;
        // this is safe beacuse the traversable trait does not allow mutation of the position of elements, and within elements the access is disjoint.
        (
            unsafe { &mut *raw }.map(|n| &n.state, |n| &mut n.state),
            unsafe { &mut *raw }.map(|n| &n.node_data, |n| &mut n.node_data),
        )
    }
}

impl<S: State> Index<ElementId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: ElementId) -> &Self::Output {
        self.get(GlobalNodeId::VNodeId(idx)).unwrap()
    }
}

impl<S: State> Index<GlobalNodeId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: GlobalNodeId) -> &Self::Output {
        self.get(idx).unwrap()
    }
}

impl<S: State> Index<usize> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: usize) -> &Self::Output {
        if let Some(template) = self.current_template() {
            template.nodes[idx].as_ref().unwrap()
        } else {
            &self[GlobalNodeId::VNodeId(dioxus_core::ElementId(idx))]
        }
    }
}

impl<S: State> IndexMut<ElementId> for RealDom<S> {
    fn index_mut(&mut self, idx: ElementId) -> &mut Self::Output {
        self.get_mut(GlobalNodeId::VNodeId(idx)).unwrap()
    }
}

impl<S: State> IndexMut<GlobalNodeId> for RealDom<S> {
    fn index_mut(&mut self, idx: GlobalNodeId) -> &mut Self::Output {
        self.get_mut(idx).unwrap()
    }
}

impl<S: State> IndexMut<usize> for RealDom<S> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        if self.template_stack.is_empty() {
            &mut self[GlobalNodeId::VNodeId(dioxus_core::ElementId(idx))]
        } else {
            self.current_template_mut()
                .unwrap()
                .nodes
                .get_mut(idx)
                .unwrap()
                .as_mut()
                .unwrap()
        }
    }
}

/// The node is stored client side and stores only basic data about the node.
#[derive(Debug, Clone)]
pub struct Node<S: State> {
    /// The transformed state of the node.
    pub state: S,
    /// The raw data for the node
    pub node_data: NodeData,
}

#[derive(Debug, Clone)]
pub struct NodeData {
    /// The id of the node this node was created from.
    pub id: ElementId,
    /// The parent id of the node.
    pub parent: Option<GlobalNodeId>,
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
        attributes: FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue>,
        listeners: Vec<&'static str>,
        children: Vec<GlobalNodeId>,
    },
    Placeholder,
}

impl<S: State> Node<S> {
    fn new(id: u64, node_type: NodeType) -> Self {
        Node {
            state: S::default(),
            node_data: NodeData {
                id: ElementId(id as usize),
                parent: None,
                node_type,
                height: 0,
            },
        }
    }

    /// Returns a reference to the element that this node refrences.
    pub fn element<'b>(&self, vdom: &'b VirtualDom) -> &'b VNode<'b> {
        vdom.get_element(self.node_data.id).unwrap()
    }

    fn add_child(&mut self, child: GlobalNodeId) {
        if let NodeType::Element { children, .. } = &mut self.node_data.node_type {
            children.push(child);
        }
    }

    fn remove_child(&mut self, child: GlobalNodeId) {
        if let NodeType::Element { children, .. } = &mut self.node_data.node_type {
            children.retain(|c| c != &child);
        }
    }

    fn set_parent(&mut self, parent: GlobalNodeId) {
        self.node_data.parent = Some(parent);
    }
}

impl<T: State> Traversable for RealDom<T> {
    type Id = GlobalNodeId;
    type Node = Node<T>;

    fn height(&self, id: Self::Id) -> Option<u16> {
        Some(<Self as Traversable>::get(self, id)?.node_data.height)
    }

    fn get(&self, id: Self::Id) -> Option<&Self::Node> {
        match id {
            GlobalNodeId::VNodeId(id) => match self.nodes.get(id.0)?.as_ref()?.as_ref() {
                TemplateRefOrNode::Ref { .. } => panic!("Template nodes should not be indexable"),
                TemplateRefOrNode::Node(n) => Some(n),
            },
            GlobalNodeId::TemplateId {
                template_ref_id,
                template_node_id,
            } => {
                let nodes = match self.nodes.get(template_ref_id.0)?.as_ref()?.as_ref() {
                    TemplateRefOrNode::Ref { nodes, .. } => nodes,
                    TemplateRefOrNode::Node(_) => panic!("Expected template ref"),
                };

                nodes
                    .get(template_node_id.0)
                    .map(|n| n.as_ref())
                    .flatten()
                    .map(|n| n.as_ref())
            }
        }
    }

    fn get_mut(&mut self, id: Self::Id) -> Option<&mut Self::Node> {
        match id {
            GlobalNodeId::VNodeId(id) => match self.nodes.get_mut(id.0)?.as_mut()?.as_mut() {
                TemplateRefOrNode::Ref { .. } => panic!("Template nodes should not be indexable"),
                TemplateRefOrNode::Node(n) => Some(n),
            },
            GlobalNodeId::TemplateId {
                template_ref_id,
                template_node_id,
            } => {
                let nodes = match self.nodes.get_mut(template_ref_id.0)?.as_mut()?.as_mut() {
                    TemplateRefOrNode::Ref { nodes, .. } => nodes,
                    TemplateRefOrNode::Node(_) => panic!("Expected template ref"),
                };

                nodes
                    .get_mut(template_node_id.0)
                    .map(|n| n.as_mut())
                    .flatten()
                    .map(|n| n.as_mut())
            }
        }
    }

    fn children(&self, id: Self::Id) -> &[Self::Id] {
        if let Some(node) = <Self as Traversable>::get(self, id) {
            match &node.node_data.node_type {
                NodeType::Element { children, .. } => &children,
                _ => &[],
            }
        } else {
            &[]
        }
    }

    fn parent(&self, id: Self::Id) -> Option<Self::Id> {
        <Self as Traversable>::get(self, id).and_then(|n| n.node_data.parent)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OwnedAttributeDiscription {
    pub name: String,
    pub namespace: Option<String>,
    pub volatile: bool,
}

impl PartialEq<AttributeDiscription> for OwnedAttributeDiscription {
    fn eq(&self, other: &AttributeDiscription) -> bool {
        self.name == other.name
            && match (&self.namespace, other.namespace) {
                (Some(a), Some(b)) => a == b,
                (None, None) => true,
                _ => false,
            }
            && self.volatile == other.volatile
    }
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Debug)]
pub struct OwnedAttributeView<'a> {
    /// The discription of the attribute.
    pub attribute: &'a OwnedAttributeDiscription,

    /// The value of the attribute.
    pub value: &'a OwnedAttributeValue,
}

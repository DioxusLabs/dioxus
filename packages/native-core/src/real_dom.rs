use anymap::AnyMap;
use fxhash::{FxHashMap, FxHashSet};
use std::ops::{Index, IndexMut};

use dioxus_core::{
    AttributeDiscription, ElementId, GlobalNodeId, Mutations, OwnedAttributeValue,
    RendererTemplateId, TemplateNodeId, VNode, JS_MAX_INT,
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
                    GlobalNodeId::VNodeId(ElementId(0)),
                    NodeType::Element {
                        tag: "Root".to_string(),
                        namespace: Some("Root"),
                        attributes: FxHashMap::default(),
                        listeners: FxHashSet::default(),
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

    /// Updates the dom with some mutations and return a set of nodes that were updated. Pass the dirty nodes to update_state.
    pub fn apply_mutations(
        &mut self,
        mutations_vec: Vec<Mutations>,
    ) -> Vec<(GlobalNodeId, NodeMask)> {
        let mut nodes_updated = Vec::new();
        nodes_updated.push((GlobalNodeId::VNodeId(ElementId(0)), NodeMask::ALL));
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
                            self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                            self.link_child(id, target).unwrap();
                        }
                    }
                    ReplaceWith { root, m } => {
                        let id = self.decode_id(root);
                        let root = self.remove(id).unwrap();
                        let target = root.parent().unwrap();
                        let drained: Vec<_> = self
                            .node_stack
                            .drain(self.node_stack.len() - m as usize..)
                            .collect();
                        for id in drained {
                            self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                            self.link_child(id, target).unwrap();
                        }
                    }
                    InsertAfter { root, n } => {
                        let target = self.parent(self.decode_id(root)).unwrap();
                        let drained: Vec<_> = self
                            .node_stack
                            .drain(self.node_stack.len() - n as usize..)
                            .collect();
                        for id in drained {
                            self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                            self.link_child(id, target).unwrap();
                        }
                    }
                    InsertBefore { root, n } => {
                        let target = self.parent(self.decode_id(root)).unwrap();
                        let drained: Vec<_> = self
                            .node_stack
                            .drain(self.node_stack.len() - n as usize..)
                            .collect();
                        for id in drained {
                            self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                            self.link_child(id, target).unwrap();
                        }
                    }
                    Remove { root } => {
                        if let Some(parent) = self.parent(self.decode_id(root)) {
                            self.mark_dirty(parent, NodeMask::NONE, &mut nodes_updated);
                        }
                        let id = self.decode_id(root);
                        self.remove(id).unwrap();
                    }
                    CreateTextNode { root, text } => {
                        let root = self.decode_id(root);
                        let n = Node::new(
                            root,
                            NodeType::Text {
                                text: text.to_string(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(root)
                    }
                    CreateTextNodeTemplate {
                        root,
                        text,
                        locally_static: _,
                    } => {
                        let root = self.decode_id(root);
                        let n = Node::new(
                            root,
                            NodeType::Text {
                                text: text.to_string(),
                            },
                        );
                        self.current_template_mut().unwrap().insert(n);
                        self.node_stack.push(root)
                    }
                    CreateElement { root, tag } => {
                        let root = self.decode_id(root);
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: None,
                                attributes: FxHashMap::default(),
                                listeners: FxHashSet::default(),
                                children: Vec::new(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(root)
                    }
                    CreateElementTemplate {
                        root,
                        tag,
                        locally_static: _,
                        fully_static: _,
                    } => {
                        let root = self.decode_id(root);
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: None,
                                attributes: FxHashMap::default(),
                                listeners: FxHashSet::default(),
                                children: Vec::new(),
                            },
                        );
                        self.current_template_mut().unwrap().insert(n);
                        self.node_stack.push(root)
                    }
                    CreateElementNs { root, tag, ns } => {
                        let root = self.decode_id(root);
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: Some(ns),
                                attributes: FxHashMap::default(),
                                listeners: FxHashSet::default(),
                                children: Vec::new(),
                            },
                        );
                        self.insert(n);
                        self.node_stack.push(root)
                    }
                    CreateElementNsTemplate {
                        root,
                        tag,
                        ns,
                        locally_static: _,
                        fully_static: _,
                    } => {
                        let root = self.decode_id(root);
                        let n = Node::new(
                            root,
                            NodeType::Element {
                                tag: tag.to_string(),
                                namespace: Some(ns),
                                attributes: FxHashMap::default(),
                                listeners: FxHashSet::default(),
                                children: Vec::new(),
                            },
                        );
                        self.current_template_mut().unwrap().insert(n);
                        self.node_stack.push(root)
                    }
                    CreatePlaceholder { root } => {
                        let root = self.decode_id(root);
                        let n = Node::new(root, NodeType::Placeholder);
                        self.insert(n);
                        self.node_stack.push(root)
                    }
                    CreatePlaceholderTemplate { root } => {
                        let root = self.decode_id(root);
                        let n = Node::new(root, NodeType::Placeholder);
                        self.current_template_mut().unwrap().insert(n);
                        self.node_stack.push(root)
                    }
                    NewEventListener {
                        event_name,
                        scope: _,
                        root,
                    } => {
                        let id = self.decode_id(root);
                        self.mark_dirty(id, NodeMask::new().with_listeners(), &mut nodes_updated);
                        match &mut self[id].node_data.node_type {
                            NodeType::Text { .. } => panic!("Text nodes cannot have listeners"),
                            NodeType::Element { listeners, .. } => {
                                listeners.insert(event_name.to_string());
                            }
                            NodeType::Placeholder => panic!("Placeholder cannot have listeners"),
                        }
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
                        self.mark_dirty(id, NodeMask::new().with_listeners(), &mut nodes_updated);
                        let v = self.nodes_listening.get_mut(event).unwrap();
                        v.remove(&id);
                    }
                    SetText {
                        root,
                        text: new_text,
                    } => {
                        let id = self.decode_id(root);
                        self.mark_dirty(id, NodeMask::new().with_text(), &mut nodes_updated);
                        let target = &mut self[id];
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
                        self.mark_dirty(
                            id,
                            NodeMask::new_with_attrs(AttributeMask::single(field)),
                            &mut nodes_updated,
                        );
                    }
                    RemoveAttribute {
                        root, name: field, ..
                    } => {
                        let id = self.decode_id(root);
                        self.mark_dirty(
                            id,
                            NodeMask::new_with_attrs(AttributeMask::single(field)),
                            &mut nodes_updated,
                        );
                    }
                    PopRoot {} => {
                        self.node_stack.pop();
                    }
                    CreateTemplateRef { id, template_id } => {
                        let template_id = RendererTemplateId(template_id as usize);
                        let template = self.templates.get(&template_id).unwrap();
                        let nodes = template.nodes.clone();
                        let id = ElementId(id as usize);
                        fn update_refrences<S: State>(
                            real_dom: &mut RealDom<S>,
                            nodes_updated: &mut Vec<(GlobalNodeId, NodeMask)>,
                            node_id: GlobalNodeId,
                            template_id: ElementId,
                        ) {
                            nodes_updated.push((node_id, NodeMask::ALL));
                            let node_id = if let GlobalNodeId::TemplateId {
                                template_node_id, ..
                            } = node_id
                            {
                                GlobalNodeId::TemplateId {
                                    template_ref_id: template_id,
                                    template_node_id,
                                }
                            } else {
                                node_id
                            };
                            let n = real_dom.get_mut(node_id).unwrap();
                            if let GlobalNodeId::TemplateId {
                                template_node_id, ..
                            } = n.node_data.id
                            {
                                n.node_data.id = GlobalNodeId::TemplateId {
                                    template_ref_id: template_id,
                                    template_node_id,
                                };
                                if let Some(GlobalNodeId::TemplateId {
                                    template_ref_id: ElementId(0),
                                    template_node_id,
                                }) = n.node_data.parent
                                {
                                    n.node_data.parent = Some(GlobalNodeId::TemplateId {
                                        template_ref_id: template_id,
                                        template_node_id,
                                    });
                                }
                            }
                            if let NodeType::Element { children, .. } = &mut n.node_data.node_type {
                                for c in children.iter_mut() {
                                    if let GlobalNodeId::TemplateId {
                                        template_node_id, ..
                                    } = c
                                    {
                                        *c = GlobalNodeId::TemplateId {
                                            template_ref_id: template_id,
                                            template_node_id: *template_node_id,
                                        };
                                    } else {
                                        panic!("non-template node in template");
                                    }
                                }
                                for c in children.clone() {
                                    update_refrences(real_dom, nodes_updated, c, template_id);
                                }
                            }
                        }
                        let template = self.templates.get(&template_id).unwrap();
                        let roots: Vec<_> = template
                            .roots
                            .iter()
                            .map(|n| GlobalNodeId::TemplateId {
                                template_ref_id: id,
                                template_node_id: TemplateNodeId(*n),
                            })
                            .collect();
                        let template_ref = TemplateRefOrNode::Ref {
                            nodes,
                            roots: roots.clone(),
                            parent: None,
                        };
                        self.resize_to(id.0);
                        self.nodes[id.0] = Some(Box::new(template_ref));
                        for node_id in roots {
                            update_refrences(self, &mut nodes_updated, node_id, id);
                        }
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

        // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
        nodes_updated.retain(|n| match &n.0 {
            GlobalNodeId::TemplateId {
                template_ref_id,
                template_node_id,
            } => self
                .nodes
                .get(template_ref_id.0)
                .and_then(|o| o.as_ref())
                .and_then(|t| match &**t {
                    TemplateRefOrNode::Ref { nodes, .. } => {
                        nodes.get(template_node_id.0).and_then(|o| o.as_ref())
                    }
                    TemplateRefOrNode::Node(_) => None,
                })
                .is_some(),
            GlobalNodeId::VNodeId(n) => self
                .nodes
                .get(n.0)
                .and_then(|o| o.as_ref())
                .and_then(|n| match &**n {
                    TemplateRefOrNode::Ref { .. } => None,
                    TemplateRefOrNode::Node(_) => Some(n),
                })
                .is_some(),
        });

        nodes_updated
    }

    fn mark_dirty(
        &self,
        gid: GlobalNodeId,
        mask: NodeMask,
        dirty_nodes: &mut Vec<(GlobalNodeId, NodeMask)>,
    ) {
        if self.template_in_progress.is_some() {
            return;
        }
        if let GlobalNodeId::VNodeId(id) = gid {
            if let TemplateRefOrNode::Ref { roots, .. } = &**self.nodes[id.0].as_ref().unwrap() {
                for r in roots {
                    dirty_nodes.push((*r, mask.clone()));
                }
            } else {
                dirty_nodes.push((gid, mask));
            }
        } else {
            dirty_nodes.push((gid, mask));
        }
    }

    fn current_template_mut(&mut self) -> Option<&mut NativeTemplate<S>> {
        self.templates.get_mut(self.template_in_progress.as_ref()?)
    }

    fn current_template(&self) -> Option<&NativeTemplate<S>> {
        self.templates.get(self.template_in_progress.as_ref()?)
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state(
        &mut self,
        nodes_updated: Vec<(GlobalNodeId, NodeMask)>,
        ctx: AnyMap,
    ) -> FxHashSet<GlobalNodeId> {
        let (mut state_tree, node_tree) = self.split();
        S::update(&nodes_updated, &mut state_tree, &node_tree, &ctx)
    }

    /// Link a child and parent together
    fn link_child(&mut self, child_id: GlobalNodeId, parent_id: GlobalNodeId) -> Option<()> {
        if let GlobalNodeId::VNodeId(id) = parent_id {
            if let TemplateRefOrNode::Ref { .. } = &**self.nodes[id.0].as_ref().unwrap() {
                return Some(());
            }
        }
        let mut created = false;
        if let GlobalNodeId::VNodeId(id) = child_id {
            #[allow(clippy::transmute_ptr_to_ref)]
            let unbounded_self: &mut Self = unsafe { std::mem::transmute(&*self as *const Self) };
            if let TemplateRefOrNode::Ref { roots, .. } = &**self.nodes[id.0].as_mut()? {
                // this is safe because we know that no parent will be it's own child
                let parent = &mut unbounded_self[parent_id];
                for r in roots {
                    parent.add_child(*r);
                }
                created = true;
            }
        }
        let parent = &mut self[parent_id];
        if !created {
            parent.add_child(child_id);
        }
        let parent_height = parent.node_data.height + 1;
        match child_id {
            GlobalNodeId::VNodeId(id) => {
                match &mut **self.nodes.get_mut(id.0).unwrap().as_mut().unwrap() {
                    TemplateRefOrNode::Ref { roots, parent, .. } => {
                        *parent = Some(parent_id);
                        for r in roots.clone() {
                            self[r].node_data.parent = Some(parent_id);
                        }
                    }
                    TemplateRefOrNode::Node(n) => n.node_data.parent = Some(parent_id),
                }
            }
            GlobalNodeId::TemplateId {
                template_ref_id,
                template_node_id,
            } => {
                let n = if let Some(template) = self.current_template_mut() {
                    &mut **template.nodes[template_node_id.0].as_mut().unwrap()
                } else {
                    let nodes = match self
                        .nodes
                        .get_mut(template_ref_id.0)
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .as_mut()
                    {
                        TemplateRefOrNode::Ref { nodes, .. } => nodes,
                        TemplateRefOrNode::Node(_) => panic!("Expected template ref"),
                    };
                    nodes
                        .get_mut(template_node_id.0)
                        .and_then(|n| n.as_mut())
                        .map(|n| n.as_mut())
                        .unwrap()
                };

                n.set_parent(parent_id);
            }
        }
        self.set_height(child_id, parent_height);

        Some(())
    }

    /// Recursively increase the height of a node and its children
    fn set_height(&mut self, id: GlobalNodeId, height: u16) {
        match id {
            GlobalNodeId::VNodeId(id) => {
                let n = &mut **self.nodes.get_mut(id.0).unwrap().as_mut().unwrap();
                match n {
                    TemplateRefOrNode::Ref { roots, .. } => {
                        for root in roots.clone() {
                            self.set_height(root, height);
                        }
                    }
                    TemplateRefOrNode::Node(n) => {
                        n.node_data.height = height;
                        if let NodeType::Element { children, .. } = &n.node_data.node_type {
                            for c in children.clone() {
                                self.set_height(c, height + 1);
                            }
                        }
                    }
                }
            }
            GlobalNodeId::TemplateId {
                template_ref_id,
                template_node_id,
            } => {
                let n = if let Some(template) = self.current_template_mut() {
                    &mut **template.nodes[template_node_id.0].as_mut().unwrap()
                } else {
                    let nodes = match self
                        .nodes
                        .get_mut(template_ref_id.0)
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .as_mut()
                    {
                        TemplateRefOrNode::Ref { nodes, .. } => nodes,
                        TemplateRefOrNode::Node(_) => panic!("Expected template ref"),
                    };
                    nodes
                        .get_mut(template_node_id.0)
                        .and_then(|n| n.as_mut())
                        .map(|n| n.as_mut())
                        .unwrap()
                };

                n.node_data.height = height;
                if let NodeType::Element { children, .. } = &n.node_data.node_type {
                    for c in children.clone() {
                        self.set_height(c, height + 1);
                    }
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
                    if let TemplateRefOrNode::Ref { nodes, roots, .. } = template_ref.as_mut() {
                        roots.retain(|r| *r != id);
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
                if let TemplateRefOrNode::Ref { nodes, roots, .. } = template_ref.as_mut() {
                    roots.retain(|r| *r != id);
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

    fn resize_to(&mut self, id: usize) {
        let current_len = self.nodes.len();
        if current_len - 1 < id {
            self.nodes.extend((0..1 + id - current_len).map(|_| None));
        }
    }

    fn insert(&mut self, node: Node<S>) {
        match node.node_data.id {
            GlobalNodeId::TemplateId { .. } => panic!("cannot insert into template"),
            GlobalNodeId::VNodeId(id) => {
                self.resize_to(id.0);
                self.nodes[id.0] = Some(Box::new(TemplateRefOrNode::Node(node)));
            }
        }
    }

    /// Find all nodes that are listening for an event, sorted by there height in the dom progressing starting at the bottom and progressing up.
    /// This can be useful to avoid creating duplicate events.
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
                    let dom_node = &self[id];
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
                                            == e.id.get().map(GlobalNodeId::VNodeId)
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
        let mut id = id.into();
        if id >= JS_MAX_INT / 2 {
            id -= JS_MAX_INT / 2;
            if self.current_template().is_some() {
                GlobalNodeId::TemplateId {
                    template_ref_id: ElementId(0),
                    template_node_id: TemplateNodeId(id as usize),
                }
            } else {
                let template_ref_id = *self.template_stack.last().unwrap();
                let template_node_id = TemplateNodeId(id as usize);
                GlobalNodeId::TemplateId {
                    template_ref_id,
                    template_node_id,
                }
            }
        } else {
            GlobalNodeId::VNodeId(ElementId(id as usize))
        }
    }

    pub fn split(
        &mut self,
    ) -> (
        impl Traversable<Id = GlobalNodeId, Node = S> + '_,
        impl Traversable<Id = GlobalNodeId, Node = NodeData> + '_,
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
    pub id: GlobalNodeId,
    /// The parent id of the node.
    pub parent: Option<GlobalNodeId>,
    /// Additional inforation specific to the node type
    pub node_type: NodeType,
    /// The number of parents before the root node. The root node has height 1.
    pub height: u16,
}

/// A type of node with data specific to the node type. The types are a subset of the [VNode] types.
#[derive(Debug, Clone)]
pub enum NodeType {
    Text {
        text: String,
    },
    Element {
        tag: String,
        namespace: Option<&'static str>,
        attributes: FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue>,
        listeners: FxHashSet<String>,
        children: Vec<GlobalNodeId>,
    },
    Placeholder,
}

impl<S: State> Node<S> {
    fn new(id: GlobalNodeId, node_type: NodeType) -> Self {
        Node {
            state: S::default(),
            node_data: NodeData {
                id,
                parent: None,
                node_type,
                height: 0,
            },
        }
    }

    /// link a child node
    fn add_child(&mut self, child: GlobalNodeId) {
        if let NodeType::Element { children, .. } = &mut self.node_data.node_type {
            children.push(child);
        }
    }

    /// remove a child node
    fn remove_child(&mut self, child: GlobalNodeId) {
        if let NodeType::Element { children, .. } = &mut self.node_data.node_type {
            children.retain(|c| c != &child);
        }
    }

    /// link the parent node
    fn set_parent(&mut self, parent: GlobalNodeId) {
        self.node_data.parent = Some(parent);
    }
}

impl<T: State> Traversable for RealDom<T> {
    type Id = GlobalNodeId;
    type Node = Node<T>;

    fn height(&self, id: Self::Id) -> Option<u16> {
        let node = <Self as Traversable>::get(self, id);
        Some(node?.node_data.height)
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
                if self.template_in_progress.is_some() {
                    let template = self.current_template().unwrap();
                    template.nodes[template_node_id.0]
                        .as_ref()
                        .map(|n| n.as_ref())
                } else {
                    let nodes = match self.nodes.get(template_ref_id.0)?.as_ref()?.as_ref() {
                        TemplateRefOrNode::Ref { nodes, .. } => nodes,
                        TemplateRefOrNode::Node(_) => {
                            panic!("Expected template ref")
                        }
                    };

                    nodes
                        .get(template_node_id.0)
                        .and_then(|n| n.as_ref())
                        .map(|n| n.as_ref())
                }
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
                if self.template_in_progress.is_some() {
                    let template = self.current_template_mut().unwrap();
                    template.nodes[template_node_id.0]
                        .as_mut()
                        .map(|n| n.as_mut())
                } else {
                    let nodes = match self.nodes.get_mut(template_ref_id.0)?.as_mut()?.as_mut() {
                        TemplateRefOrNode::Ref { nodes, .. } => nodes,
                        TemplateRefOrNode::Node(_) => panic!("Expected template ref"),
                    };

                    nodes
                        .get_mut(template_node_id.0)
                        .and_then(|n| n.as_mut())
                        .map(|n| n.as_mut())
                }
            }
        }
    }

    fn children(&self, id: Self::Id) -> &[Self::Id] {
        if let Some(node) = <Self as Traversable>::get(self, id) {
            match &node.node_data.node_type {
                NodeType::Element { children, .. } => children,
                _ => &[],
            }
        } else {
            &[]
        }
    }

    fn parent(&self, id: Self::Id) -> Option<Self::Id> {
        match id {
            GlobalNodeId::VNodeId(id) => self.nodes.get(id.0).as_ref()?.as_ref()?.parent(),
            GlobalNodeId::TemplateId {
                template_ref_id,
                template_node_id,
            } => {
                if self.template_in_progress.is_some() {
                    let template = self.current_template().unwrap();
                    template.nodes[template_node_id.0]
                        .as_ref()
                        .map(|n| n.as_ref())
                } else {
                    let nodes = match self.nodes.get(template_ref_id.0)?.as_ref()?.as_ref() {
                        TemplateRefOrNode::Ref { nodes, .. } => nodes,
                        TemplateRefOrNode::Node(_) => panic!("Expected template ref"),
                    };

                    nodes.get(template_node_id.0).and_then(|n| n.as_deref())
                }?
                .node_data
                .parent
            }
        }
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

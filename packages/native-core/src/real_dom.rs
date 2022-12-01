use anymap::AnyMap;
use dioxus_core::{AttributeDiscription, ElementId, Mutations, OwnedAttributeValue, VNode};
use rustc_hash::{FxHashMap, FxHashSet};
use slab::Slab;
use std::ops::{Index, IndexMut};

use crate::node_ref::{AttributeMask, NodeMask};
use crate::state::State;
use crate::traversable::Traversable;
use crate::RealNodeId;

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [crate::state::ParentDepState], [crate::state::NodeDepState], or [crate::state::ChildDepState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
#[derive(Debug)]
pub struct RealDom<S: State> {
    root: usize,
    nodes: Vec<Option<Box<Node<S>>>>,
    // some nodes do not have an ElementId immediately, those node are stored here
    internal_nodes: Slab<Box<Node<S>>>,
    nodes_listening: FxHashMap<&'static str, FxHashSet<RealNodeId>>,
    last: Option<RealNodeId>,
    // any nodes that have children queued to be added in the form (parent, children remaining)
    parents_queued: Vec<(RealNodeId, u32)>,
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
            namespace: Some("Root"),
            attributes: FxHashMap::default(),
            listeners: FxHashSet::default(),
            children: Vec::new(),
        });
        root.node_data.id = Some(RealNodeId::ElementId(ElementId(0)));

        RealDom {
            root: 0,
            nodes: vec![Some(Box::new(root))],
            internal_nodes: Slab::new(),
            nodes_listening: FxHashMap::default(),
            last: None,
            parents_queued: Vec::new(),
        }
    }

    pub fn resolve_maybe_id(&self, id: Option<u64>) -> RealNodeId {
        if let Some(id) = id {
            RealNodeId::ElementId(ElementId(id as usize))
        } else {
            self.last.unwrap()
        }
    }

    /// Updates the dom with some mutations and return a set of nodes that were updated. Pass the dirty nodes to update_state.
    pub fn apply_mutations(
        &mut self,
        mutations_vec: Vec<Mutations>,
    ) -> Vec<(RealNodeId, NodeMask)> {
        let mut nodes_updated = Vec::new();
        nodes_updated.push((RealNodeId::ElementId(ElementId(0)), NodeMask::ALL));
        for mutations in mutations_vec {
            for e in mutations.dom_edits {
                use dioxus_core::DomEdit::*;
                match e {
                    AppendChildren { root, children } => {
                        let target = self.resolve_maybe_id(root);
                        for id in children {
                            let id = RealNodeId::ElementId(ElementId(id as usize));
                            self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                            self.link_child(id, target).unwrap();
                        }
                    }
                    ReplaceWith { root, nodes } => {
                        let id_to_replace = self.resolve_maybe_id(root);
                        let target = self[id_to_replace].node_data.parent.unwrap();
                        for id in nodes {
                            let id = RealNodeId::ElementId(ElementId(id as usize));
                            self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                            self.link_child_before(id, target, id_to_replace).unwrap();
                        }
                        self.remove(id_to_replace).unwrap();
                    }
                    InsertAfter { root, nodes } => {
                        let root = self.resolve_maybe_id(root);
                        let target = self.parent(root).unwrap();
                        for id in nodes {
                            let id = RealNodeId::ElementId(ElementId(id as usize));
                            self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                            self.link_child_after(id, target, root).unwrap();
                        }
                    }
                    InsertBefore { root, nodes } => {
                        let root = self.resolve_maybe_id(root);
                        let target = self.parent(root).unwrap();
                        for id in nodes {
                            let id = RealNodeId::ElementId(ElementId(id as usize));
                            self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                            self.link_child_before(id, target, root).unwrap();
                        }
                    }
                    Remove { root } => {
                        let id = self.resolve_maybe_id(root);
                        if let Some(parent) = self.parent(id) {
                            self.mark_dirty(parent, NodeMask::NONE, &mut nodes_updated);
                        }
                        self.remove(id).unwrap();
                    }
                    CreateTextNode { root, text } => {
                        let n = Node::new(NodeType::Text {
                            text: text.to_string(),
                        });
                        let id = self.insert(n, root, &mut nodes_updated);
                        self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                        if let Some((parent, remaining)) = self.parents_queued.last_mut() {
                            *remaining -= 1;
                            let parent = *parent;
                            if *remaining == 0 {
                                self.parents_queued.pop();
                            }
                            self.link_child(id, parent).unwrap();
                        }
                        self.last = Some(id);
                    }
                    CreateElement {
                        root,
                        tag,
                        children,
                    } => {
                        let n = Node::new(NodeType::Element {
                            tag: tag.to_string(),
                            namespace: None,
                            attributes: FxHashMap::default(),
                            listeners: FxHashSet::default(),
                            children: Vec::new(),
                        });
                        let id = self.insert(n, root, &mut nodes_updated);
                        self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                        if let Some((parent, remaining)) = self.parents_queued.last_mut() {
                            *remaining -= 1;
                            let parent = *parent;
                            if *remaining == 0 {
                                self.parents_queued.pop();
                            }
                            self.link_child(id, parent).unwrap();
                        }
                        self.last = Some(id);
                        if children > 0 {
                            self.parents_queued.push((id, children));
                        }
                    }
                    CreateElementNs {
                        root,
                        tag,
                        ns,
                        children,
                    } => {
                        let n = Node::new(NodeType::Element {
                            tag: tag.to_string(),
                            namespace: Some(ns),
                            attributes: FxHashMap::default(),
                            listeners: FxHashSet::default(),
                            children: Vec::new(),
                        });
                        let id = self.insert(n, root, &mut nodes_updated);
                        self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                        if let Some((parent, remaining)) = self.parents_queued.last_mut() {
                            *remaining -= 1;
                            let parent = *parent;
                            if *remaining == 0 {
                                self.parents_queued.pop();
                            }
                            self.link_child(id, parent).unwrap();
                        }
                        self.last = Some(id);
                        if children > 0 {
                            self.parents_queued.push((id, children));
                        }
                    }
                    CreatePlaceholder { root } => {
                        let n = Node::new(NodeType::Placeholder);
                        let id = self.insert(n, root, &mut nodes_updated);
                        self.mark_dirty(id, NodeMask::ALL, &mut nodes_updated);
                        if let Some((parent, remaining)) = self.parents_queued.last_mut() {
                            *remaining -= 1;
                            let parent = *parent;
                            if *remaining == 0 {
                                self.parents_queued.pop();
                            }
                            self.link_child(id, parent).unwrap();
                        }
                        self.last = Some(id);
                    }
                    NewEventListener {
                        event_name,
                        scope: _,
                        root,
                    } => {
                        let id = self.resolve_maybe_id(root);
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
                        let id = self.resolve_maybe_id(root);
                        self.mark_dirty(id, NodeMask::new().with_listeners(), &mut nodes_updated);
                        let v = self.nodes_listening.get_mut(event).unwrap();
                        v.remove(&id);
                    }
                    SetText {
                        root,
                        text: new_text,
                    } => {
                        let id = self.resolve_maybe_id(root);
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
                        let id = self.resolve_maybe_id(root);
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
                        let id = self.resolve_maybe_id(root);
                        self.mark_dirty(
                            id,
                            NodeMask::new_with_attrs(AttributeMask::single(field)),
                            &mut nodes_updated,
                        );
                    }
                    CloneNode { id, new_id } => {
                        let id = self.resolve_maybe_id(id);
                        self.clone_node_into(id, &mut nodes_updated, Some(new_id));
                    }
                    CloneNodeChildren { id, new_ids } => {
                        let id = self.resolve_maybe_id(id);
                        let bounded_self: &mut Self = self;
                        let unbounded_self: &mut Self =
                            unsafe { std::mem::transmute(bounded_self) };
                        if let NodeType::Element { children, .. } = &self[id].node_data.node_type {
                            for (old_id, new_id) in children.iter().zip(new_ids) {
                                let child_id = unbounded_self.clone_node_into(
                                    *old_id,
                                    &mut nodes_updated,
                                    Some(new_id),
                                );
                                unbounded_self[child_id].node_data.parent = None;
                            }
                        }
                    }
                    FirstChild {} => {
                        if let NodeType::Element { children, .. } =
                            &self[self.last.unwrap()].node_data.node_type
                        {
                            self.last = Some(children[0]);
                        } else {
                            panic!("tried to call first child on a non element");
                        }
                    }
                    NextSibling {} => {
                        let id = self.last.unwrap();
                        if let Some(parent) = self.parent(id) {
                            if let NodeType::Element { children, .. } =
                                &self[parent].node_data.node_type
                            {
                                let index = children.iter().position(|a| *a == id).unwrap();
                                self.last = Some(children[index + 1]);
                            }
                        } else {
                            panic!("tried to call next sibling on a non element");
                        }
                    }
                    ParentNode {} => {
                        if let Some(parent) = self.parent(self.last.unwrap()) {
                            self.last = Some(parent);
                        } else {
                            panic!("tried to call parent node on a non element");
                        }
                    }
                    StoreWithId { id } => {
                        let old_id = self.last.unwrap();
                        let node = self.internal_nodes.remove(old_id.as_unaccessable_id());
                        let new_id = self.insert(*node, Some(id), &mut nodes_updated);
                        self.update_id(old_id, new_id, &mut nodes_updated);
                    }
                    SetLastNode { id } => {
                        self.last =
                            Some(RealNodeId::ElementId(dioxus_core::ElementId(id as usize)));
                    }
                }
            }
        }

        // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
        nodes_updated.retain(|n| match n.0 {
            RealNodeId::ElementId(id) => self.nodes.get(id.0).and_then(|o| o.as_ref()).is_some(),
            RealNodeId::UnaccessableId(id) => self.internal_nodes.get(id).is_some(),
        });

        nodes_updated
    }

    /// Update refrences to an old node id to a new node id
    fn update_id(
        &mut self,
        old_id: RealNodeId,
        new_id: RealNodeId,
        nodes_updated: &mut Vec<(RealNodeId, NodeMask)>,
    ) {
        // this is safe because a node cannot have itself as a child or parent
        let unbouned_self = unsafe { &mut *(self as *mut Self) };
        // update parent's link to child id
        if let Some(parent) = self[new_id].node_data.parent {
            if let NodeType::Element { children, .. } = &mut self[parent].node_data.node_type {
                for c in children {
                    if *c == old_id {
                        *c = new_id;
                        break;
                    }
                }
            }
        }
        // update child's link to parent id
        if let NodeType::Element { children, .. } = &self[new_id].node_data.node_type {
            for child_id in children {
                unbouned_self[*child_id].node_data.parent = Some(new_id);
            }
        }
        // update dirty nodes
        for (node, _) in nodes_updated {
            if *node == old_id {
                *node = new_id;
            }
        }
        // update nodes listening
        for v in self.nodes_listening.values_mut() {
            if v.contains(&old_id) {
                v.remove(&old_id);
                v.insert(new_id);
            }
        }
        // update last
        if let Some(last) = self.last {
            if last == old_id {
                self.last = Some(new_id);
            }
        }
    }

    fn mark_dirty(
        &self,
        gid: RealNodeId,
        mask: NodeMask,
        dirty_nodes: &mut Vec<(RealNodeId, NodeMask)>,
    ) {
        dirty_nodes.push((gid, mask));
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state(
        &mut self,
        nodes_updated: Vec<(RealNodeId, NodeMask)>,
        ctx: AnyMap,
    ) -> FxHashSet<RealNodeId> {
        let (mut state_tree, node_tree) = self.split();
        S::update(&nodes_updated, &mut state_tree, &node_tree, &ctx)
    }

    /// Link a child and parent together
    fn link_child(&mut self, child_id: RealNodeId, parent_id: RealNodeId) -> Option<()> {
        let parent = &mut self[parent_id];
        parent.add_child(child_id);
        let parent_height = parent.node_data.height;
        self[child_id].set_parent(parent_id);
        self.set_height(child_id, parent_height + 1);

        Some(())
    }

    /// Link a child and parent together with the child inserted before a marker
    fn link_child_before(
        &mut self,
        child_id: RealNodeId,
        parent_id: RealNodeId,
        marker: RealNodeId,
    ) -> Option<()> {
        let parent = &mut self[parent_id];
        if let NodeType::Element { children, .. } = &mut parent.node_data.node_type {
            let index = children.iter().position(|a| *a == marker)?;
            children.insert(index, child_id);
        }
        let parent_height = parent.node_data.height;
        self[child_id].set_parent(parent_id);
        self.set_height(child_id, parent_height + 1);

        Some(())
    }

    /// Link a child and parent together with the child inserted after a marker
    fn link_child_after(
        &mut self,
        child_id: RealNodeId,
        parent_id: RealNodeId,
        marker: RealNodeId,
    ) -> Option<()> {
        let parent = &mut self[parent_id];
        if let NodeType::Element { children, .. } = &mut parent.node_data.node_type {
            let index = children.iter().position(|a| *a == marker)?;
            children.insert(index + 1, child_id);
        }
        let parent_height = parent.node_data.height;
        self[child_id].set_parent(parent_id);
        self.set_height(child_id, parent_height + 1);

        Some(())
    }

    /// Recursively increase the height of a node and its children
    fn set_height(&mut self, id: RealNodeId, height: u16) {
        let node = &mut self[id];
        node.node_data.height = height;
        if let NodeType::Element { children, .. } = &node.node_data.node_type {
            for c in children.clone() {
                self.set_height(c, height + 1);
            }
        }
    }

    // remove a node and it's children from the dom.
    fn remove(&mut self, id: RealNodeId) -> Option<Node<S>> {
        // We do not need to remove the node from the parent's children list for children.
        fn inner<S: State>(dom: &mut RealDom<S>, id: RealNodeId) -> Option<Node<S>> {
            let mut node = match id {
                RealNodeId::ElementId(id) => *dom.nodes[id.0].take()?,
                RealNodeId::UnaccessableId(id) => *dom.internal_nodes.remove(id),
            };
            if let NodeType::Element { children, .. } = &mut node.node_data.node_type {
                for c in children {
                    inner(dom, *c);
                }
            }
            Some(node)
        }
        let mut node = match id {
            RealNodeId::ElementId(id) => *self.nodes[id.0].take()?,
            RealNodeId::UnaccessableId(id) => *self.internal_nodes.remove(id),
        };
        if let Some(parent) = node.node_data.parent {
            let parent = &mut self[parent];
            parent.remove_child(id);
        }
        if let NodeType::Element { children, .. } = &mut node.node_data.node_type {
            for c in children {
                inner(self, *c)?;
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

    fn insert(
        &mut self,
        mut node: Node<S>,
        id: Option<u64>,
        nodes_updated: &mut Vec<(RealNodeId, NodeMask)>,
    ) -> RealNodeId {
        match id {
            Some(id) => {
                let id = id as usize;
                self.resize_to(id);
                let real_id = RealNodeId::ElementId(ElementId(id));
                node.node_data.id = Some(real_id);
                // move the old node to a new unaccessable id
                if let Some(mut old) = self.nodes[id].take() {
                    let old_id = old.node_data.id.unwrap();
                    let entry = self.internal_nodes.vacant_entry();
                    let id = entry.key();
                    let new_id = RealNodeId::UnaccessableId(id);
                    old.node_data.id = Some(real_id);
                    entry.insert(old);
                    self.update_id(old_id, new_id, nodes_updated);
                }
                self.nodes[id] = Some(Box::new(node));
                real_id
            }
            None => {
                let entry = self.internal_nodes.vacant_entry();
                let id = entry.key();
                let real_id = RealNodeId::UnaccessableId(id);
                node.node_data.id = Some(real_id);
                entry.insert(Box::new(node));
                real_id
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
                                            .map(|c| RealNodeId::ElementId(c.mounted_id())),
                                    )
                                    .all(|(c1, c2)| *c1 == c2)
                                && e.children.iter().all(|c| {
                                    self.contains_node(c)
                                        && self[RealNodeId::ElementId(c.mounted_id())]
                                            .node_data
                                            .parent
                                            == e.id.get().map(RealNodeId::ElementId)
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
                    let dom_node = &self[RealNodeId::ElementId(id)];
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
        fn inner<S: State>(dom: &RealDom<S>, id: RealNodeId, f: &mut impl FnMut(&Node<S>)) {
            let node = &dom[id];
            f(node);
            if let NodeType::Element { children, .. } = &node.node_data.node_type {
                for c in children {
                    inner(dom, *c, f);
                }
            }
        }
        if let NodeType::Element { children, .. } = &self
            [RealNodeId::ElementId(ElementId(self.root))]
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
        fn inner<S: State>(dom: &mut RealDom<S>, id: RealNodeId, f: &mut impl FnMut(&mut Node<S>)) {
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
            [RealNodeId::ElementId(ElementId(root))]
        .node_data
        .node_type
        {
            for c in children.clone() {
                inner(self, c, &mut f);
            }
        }
    }

    pub fn split(
        &mut self,
    ) -> (
        impl Traversable<Id = RealNodeId, Node = S> + '_,
        impl Traversable<Id = RealNodeId, Node = NodeData> + '_,
    ) {
        let raw = self as *mut Self;
        // this is safe beacuse the traversable trait does not allow mutation of the position of elements, and within elements the access is disjoint.
        (
            unsafe { &mut *raw }.map(|n| &n.state, |n| &mut n.state),
            unsafe { &mut *raw }.map(|n| &n.node_data, |n| &mut n.node_data),
        )
    }

    /// Recurively clones a node and marks it and it's children as dirty.
    fn clone_node_into(
        &mut self,
        id: RealNodeId,
        nodes_updated: &mut Vec<(RealNodeId, NodeMask)>,
        new_id: Option<u64>,
    ) -> RealNodeId {
        let new_id = self.insert(self[id].clone(), new_id, nodes_updated);
        nodes_updated.push((new_id, NodeMask::ALL));
        // this is safe because no node has itself as a child.
        let unbounded_self = unsafe { &mut *(self as *mut Self) };
        let mut node = &mut self[new_id];
        node.node_data.height = 0;
        if let NodeType::Element { children, .. } = &mut node.node_data.node_type {
            for c in children {
                let child_id = unbounded_self.clone_node_into(*c, nodes_updated, None);
                *c = child_id;
                let parent_height = node.node_data.height;
                unbounded_self[child_id].set_parent(new_id);
                unbounded_self.set_height(child_id, parent_height + 1);
            }
        }
        new_id
    }
}

impl<S: State> Index<ElementId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: ElementId) -> &Self::Output {
        self.get(RealNodeId::ElementId(idx)).unwrap()
    }
}

impl<S: State> Index<RealNodeId> for RealDom<S> {
    type Output = Node<S>;

    fn index(&self, idx: RealNodeId) -> &Self::Output {
        self.get(idx).unwrap()
    }
}

impl<S: State> IndexMut<ElementId> for RealDom<S> {
    fn index_mut(&mut self, idx: ElementId) -> &mut Self::Output {
        self.get_mut(RealNodeId::ElementId(idx)).unwrap()
    }
}

impl<S: State> IndexMut<RealNodeId> for RealDom<S> {
    fn index_mut(&mut self, idx: RealNodeId) -> &mut Self::Output {
        self.get_mut(idx).unwrap()
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
    pub id: Option<RealNodeId>,
    /// The parent id of the node.
    pub parent: Option<RealNodeId>,
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
        children: Vec<RealNodeId>,
    },
    Placeholder,
}

impl<S: State> Node<S> {
    fn new(node_type: NodeType) -> Self {
        Node {
            state: S::default(),
            node_data: NodeData {
                id: None,
                parent: None,
                node_type,
                height: 0,
            },
        }
    }

    /// link a child node
    fn add_child(&mut self, child: RealNodeId) {
        if let NodeType::Element { children, .. } = &mut self.node_data.node_type {
            children.push(child);
        }
    }

    /// remove a child node
    fn remove_child(&mut self, child: RealNodeId) {
        if let NodeType::Element { children, .. } = &mut self.node_data.node_type {
            children.retain(|c| c != &child);
        }
    }

    /// link the parent node
    fn set_parent(&mut self, parent: RealNodeId) {
        self.node_data.parent = Some(parent);
    }

    /// get the mounted id of the node
    pub fn mounted_id(&self) -> RealNodeId {
        self.node_data.id.unwrap()
    }
}

impl<T: State> Traversable for RealDom<T> {
    type Id = RealNodeId;
    type Node = Node<T>;

    fn height(&self, id: Self::Id) -> Option<u16> {
        let node = <Self as Traversable>::get(self, id);
        Some(node?.node_data.height)
    }

    fn get(&self, id: Self::Id) -> Option<&Self::Node> {
        match id {
            RealNodeId::ElementId(id) => {
                self.nodes.get(id.0).and_then(|b| b.as_ref().map(|b| &**b))
            }
            RealNodeId::UnaccessableId(id) => self.internal_nodes.get(id).map(|b| &**b),
        }
    }

    fn get_mut(&mut self, id: Self::Id) -> Option<&mut Self::Node> {
        match id {
            RealNodeId::ElementId(id) => self
                .nodes
                .get_mut(id.0)
                .and_then(|b| b.as_mut().map(|b| &mut **b)),
            RealNodeId::UnaccessableId(id) => self.internal_nodes.get_mut(id).map(|b| &mut **b),
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
        self.get(id).and_then(|n| n.node_data.parent)
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

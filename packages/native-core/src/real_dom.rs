use rustc_hash::{FxHashMap, FxHashSet};
use std::any::{Any, TypeId};
use std::collections::VecDeque;

use crate::node::{
    ElementNode, FromAnyValue, NodeType, OwnedAttributeDiscription, OwnedAttributeValue,
};
use crate::node_ref::{AttributeMask, NodeMask, NodeMaskBuilder};
use crate::passes::{resolve_passes, DirtyNodeStates, TypeErasedPass};
use crate::tree::{NodeId, Tree};
use crate::{FxDashSet, SendAnyMap};

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [crate::state::ParentDepState], [crate::state::NodeDepState], or [crate::state::ChildDepState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
///
/// # Custom values
/// To allow custom values to be passed into attributes implement FromAnyValue on a type that can represent your custom value and specify the V generic to be that type. If you have many different custom values, it can be useful to use a enum type to represent the varients.
pub struct RealDom<V: FromAnyValue + Send + Sync = ()> {
    pub(crate) tree: Tree,
    nodes_listening: FxHashMap<String, FxHashSet<NodeId>>,
    pub(crate) passes: Box<[TypeErasedPass<V>]>,
    passes_updated: FxHashMap<NodeId, FxHashSet<TypeId>>,
    nodes_updated: FxHashMap<NodeId, NodeMask>,
    phantom: std::marker::PhantomData<V>,
}

impl<V: FromAnyValue + Send + Sync> RealDom<V> {
    pub fn new(mut passes: Box<[TypeErasedPass<V>]>) -> RealDom<V> {
        let mut tree = Tree::new();
        tree.insert_slab::<NodeType<V>>();
        for pass in passes.iter() {
            (pass.create)(&mut tree);
        }
        let root_id = tree.root();
        let root_node: NodeType<V> = NodeType::Element(ElementNode {
            tag: "Root".to_string(),
            namespace: Some("Root".to_string()),
            attributes: FxHashMap::default(),
            listeners: FxHashSet::default(),
        });
        tree.insert(root_id, root_node);

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

        let mut passes_updated = FxHashMap::default();
        let mut nodes_updated = FxHashMap::default();

        let root_id = NodeId(0);
        passes_updated.insert(root_id, passes.iter().map(|x| x.this_type_id).collect());
        nodes_updated.insert(root_id, NodeMaskBuilder::ALL.build());

        RealDom {
            tree,
            nodes_listening: FxHashMap::default(),
            passes,
            passes_updated,
            nodes_updated,
            phantom: std::marker::PhantomData,
        }
    }

    fn mark_dirty(&mut self, node_id: NodeId, mask: NodeMask) {
        self.passes_updated.entry(node_id).or_default().extend(
            self.passes
                .iter()
                .filter_map(|x| x.mask.overlaps(&mask).then_some(x.this_type_id)),
        );
        let nodes_updated = &mut self.nodes_updated;
        if let Some(node) = nodes_updated.get_mut(&node_id) {
            *node = node.union(&mask);
        } else {
            nodes_updated.insert(node_id, mask);
        }
    }

    fn mark_parent_added_or_removed(&mut self, node_id: NodeId) {
        let hm = self.passes_updated.entry(node_id).or_default();
        for pass in &*self.passes {
            if pass.parent_dependant {
                hm.insert(pass.this_type_id);
            }
        }
    }

    fn mark_child_changed(&mut self, node_id: NodeId) {
        let hm = self.passes_updated.entry(node_id).or_default();
        for pass in &*self.passes {
            if pass.child_dependant {
                hm.insert(pass.this_type_id);
            }
        }
    }

    pub fn create_node(&mut self, node: NodeType<V>, mark_dirty: bool) -> NodeMut<'_, V> {
        let mut node_entry = self.tree.create_node();
        let id = node_entry.id();
        if mark_dirty {
            self.passes_updated
                .entry(id)
                .or_default()
                .extend(self.passes.iter().map(|x| x.this_type_id));
        }
        node_entry.insert(node);
        NodeMut::new(id, self)
    }

    pub fn add_child(&mut self, node_id: NodeId, child_id: NodeId) {
        self.tree.add_child(node_id, child_id);
    }

    /// Find all nodes that are listening for an event, sorted by there height in the dom progressing starting at the bottom and progressing up.
    /// This can be useful to avoid creating duplicate events.
    pub fn get_listening_sorted(&self, event: &str) -> Vec<NodeRef<V>> {
        if let Some(nodes) = self.nodes_listening.get(event) {
            let mut listening: Vec<_> = nodes
                .iter()
                .map(|id| (*id, self.tree.height(*id).unwrap()))
                .collect();
            listening.sort_by(|(_, h1), (_, h2)| h1.cmp(h2).reverse());
            listening
                .into_iter()
                .map(|(id, _)| NodeRef { id, dom: self })
                .collect()
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

    pub fn clone_node(&mut self, node_id: NodeId) -> NodeId {
        let node = self.get(node_id).unwrap();
        let new_node = node.node_type().clone();
        let new_id = self.create_node(new_node, true).id();

        let children = self.tree.children_ids(node_id).unwrap().to_vec();
        for child in children {
            let child_id = self.clone_node(child);
            self.add_child(new_id, child_id);
        }
        new_id
    }

    pub fn get(&self, id: NodeId) -> Option<NodeRef<'_, V>> {
        let id = id.into_node_id(self);
        self.tree.contains(id).then_some(NodeRef { id, dom: self })
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<NodeMut<'_, V>> {
        let id = id.into_node_id(self);
        self.tree.contains(id).then(|| NodeMut::new(id, self))
    }

    /// WARNING: This escapes the reactive system that the real dom uses. Any changes made with this method will not trigger updates in states when [RealDom::update_state] is called.
    pub fn get_mut_raw(&mut self, id: NodeId) -> Option<NodeMutRaw<V>> {
        let id = id.into_node_id(self);
        self.tree
            .contains(id)
            .then_some(NodeMutRaw { id, dom: self })
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state(
        &mut self,
        ctx: SendAnyMap,
        parallel: bool,
    ) -> (FxDashSet<NodeId>, FxHashMap<NodeId, NodeMask>) {
        let passes = std::mem::take(&mut self.passes_updated);
        let nodes_updated = std::mem::take(&mut self.nodes_updated);
        let dirty_nodes = DirtyNodeStates::with_passes(self.passes.iter().map(|p| p.this_type_id));
        for (node_id, passes) in passes {
            // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
            if let Some(height) = self.tree.height(node_id) {
                for pass in passes {
                    dirty_nodes.insert(pass, node_id, height);
                }
            }
        }

        (
            resolve_passes(self, dirty_nodes, ctx, parallel),
            nodes_updated,
        )
    }

    pub fn remove(&mut self, id: NodeId) {
        if let Some(parent_id) = self.tree.parent_id(id) {
            self.mark_child_changed(parent_id);
        }
        self.tree.remove(id)
    }

    pub fn replace(&mut self, old: NodeId, new: NodeId) {
        if let Some(parent_id) = self.tree.parent_id(old) {
            self.mark_child_changed(parent_id);
            self.mark_parent_added_or_removed(new);
        }
        self.tree.replace(old, new);
    }

    pub fn traverse_depth_first(&self, mut f: impl FnMut(NodeRef<V>)) {
        let mut stack = vec![self.root_id()];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.get(id) {
                f(node);
                if let Some(children) = self.tree.children_ids(id) {
                    stack.extend(children.iter().copied().rev());
                }
            }
        }
    }

    pub fn traverse_breadth_first(&self, mut f: impl FnMut(NodeRef<V>)) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root_id());
        while let Some(id) = queue.pop_front() {
            if let Some(node) = self.get(id) {
                f(node);
                if let Some(children) = self.tree.children_ids(id) {
                    for id in children {
                        queue.push_back(*id);
                    }
                }
            }
        }
    }

    pub fn traverse_depth_first_mut(&mut self, mut f: impl FnMut(NodeMut<V>)) {
        let mut stack = vec![self.root_id()];
        while let Some(id) = stack.pop() {
            if let Some(children) = self.tree.children_ids(id) {
                let children = children.iter().copied().rev().collect::<Vec<_>>();
                if let Some(node) = self.get_mut(id) {
                    let node = node;
                    f(node);
                    stack.extend(children.iter());
                }
            }
        }
    }

    pub fn traverse_breadth_first_mut(&mut self, mut f: impl FnMut(NodeMut<V>)) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root_id());
        while let Some(id) = queue.pop_front() {
            if let Some(children) = self.tree.children_ids(id) {
                let children = children.to_vec();
                if let Some(node) = self.get_mut(id) {
                    f(node);
                    for id in children {
                        queue.push_back(id);
                    }
                }
            }
        }
    }

    pub fn insert_slab<T: Any + Send + Sync>(&mut self) {
        self.tree.insert_slab::<T>();
    }
}

pub trait IntoNodeId<V: FromAnyValue + Send + Sync> {
    fn into_node_id(self, rdom: &RealDom<V>) -> NodeId;
}

impl<V: FromAnyValue + Send + Sync> IntoNodeId<V> for NodeId {
    fn into_node_id(self, _rdom: &RealDom<V>) -> NodeId {
        self
    }
}

pub trait NodeImmutable<V: FromAnyValue + Send + Sync>: Sized {
    fn real_dom(&self) -> &RealDom<V>;

    fn id(&self) -> NodeId;

    fn node_type(&self) -> &NodeType<V> {
        self.get().unwrap()
    }

    fn get<T: Any + Sync + Send>(&self) -> Option<&T> {
        self.real_dom().tree.get(self.id())
    }

    fn child_ids(&self) -> Option<&[NodeId]> {
        self.real_dom().tree.children_ids(self.id())
    }

    fn children(&self) -> Option<Vec<NodeRef<V>>> {
        self.child_ids().map(|ids| {
            ids.iter()
                .map(|id| NodeRef {
                    id: *id,
                    dom: self.real_dom(),
                })
                .collect()
        })
    }

    fn parent_id(&self) -> Option<NodeId> {
        self.real_dom().tree.parent_id(self.id())
    }

    fn parent(&self) -> Option<NodeRef<V>> {
        self.parent_id().map(|id| NodeRef {
            id,
            dom: self.real_dom(),
        })
    }
}

pub trait NodeMutable<V: FromAnyValue + Send + Sync>: Sized + NodeImmutable<V> {
    fn real_dom_mut(&mut self) -> &mut RealDom<V>;
    fn get_mut<T: Any + Sync + Send>(&mut self) -> Option<&mut T> {
        let id = self.id();
        self.real_dom_mut().tree.get_mut(id)
    }
    fn insert<T: Any + Sync + Send>(&mut self, value: T) {
        let id = self.id();
        self.real_dom_mut().tree.insert(id, value);
    }
    fn add_child(&mut self, child: NodeId) {
        let id = self.id();
        self.real_dom_mut().tree.add_child(id, child);
    }
    fn insert_after(&mut self, old: NodeId) {
        let id = self.id();
        self.real_dom_mut().tree.insert_after(old, id);
    }
    fn insert_before(&mut self, old: NodeId) {
        let id = self.id();
        self.real_dom_mut().tree.insert_before(old, id);
    }
    fn add_event_listener(&mut self, event: &str);
    fn remove_event_listener(&mut self, event: &str);
}

#[derive(Clone, Copy)]
pub struct NodeRef<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    dom: &'a RealDom<V>,
}

impl<'a, V: FromAnyValue + Send + Sync> NodeImmutable<V> for NodeRef<'a, V> {
    fn real_dom(&self) -> &RealDom<V> {
        self.dom
    }

    fn id(&self) -> NodeId {
        self.id
    }
}

pub struct NodeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    dom: &'a mut RealDom<V>,
    dirty: NodeMask,
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMut<'a, V> {
    pub fn new(id: NodeId, dom: &'a mut RealDom<V>) -> Self {
        Self {
            id,
            dom,
            dirty: NodeMask::default(),
        }
    }
}

impl<'a, V: FromAnyValue + Send + Sync> NodeImmutable<V> for NodeMut<'a, V> {
    fn real_dom(&self) -> &RealDom<V> {
        self.dom
    }

    fn id(&self) -> NodeId {
        self.id
    }
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMutable<V> for NodeMut<'a, V> {
    fn real_dom_mut(&mut self) -> &mut RealDom<V> {
        self.dom
    }

    fn get_mut<T: Any + Sync + Send>(&mut self) -> Option<&mut T> {
        // mark the node state as dirty
        self.dom
            .passes_updated
            .entry(self.id)
            .or_default()
            .insert(TypeId::of::<T>());
        self.dom.tree.get_mut(self.id)
    }

    fn insert<T: Any + Sync + Send>(&mut self, value: T) {
        // mark the node state as dirty
        self.dom
            .passes_updated
            .entry(self.id)
            .or_default()
            .insert(TypeId::of::<T>());
        self.dom.tree.insert(self.id, value);
    }

    fn insert_after(&mut self, old: NodeId) {
        let id = self.id();
        if let Some(parent_id) = self.dom.tree.parent_id(old) {
            self.dom.mark_child_changed(parent_id);
            self.dom.mark_parent_added_or_removed(id);
        }
        self.dom.tree.insert_after(old, id);
    }

    fn insert_before(&mut self, old: NodeId) {
        let id = self.id();
        if let Some(parent_id) = self.dom.tree.parent_id(old) {
            self.dom.mark_child_changed(parent_id);
            self.dom.mark_parent_added_or_removed(id);
        }
        self.dom.tree.insert_before(old, id);
    }

    fn add_event_listener(&mut self, event: &str) {
        let id = self.id();
        if let NodeTypeMut::Element(mut element) = self.node_type_mut() {
            element.listeners_mut().insert(event.to_string());
            match self.dom.nodes_listening.get_mut(event) {
                Some(hs) => {
                    hs.insert(id);
                }
                None => {
                    let mut hs = FxHashSet::default();
                    hs.insert(id);
                    self.dom.nodes_listening.insert(event.to_string(), hs);
                }
            }
        }
    }

    fn remove_event_listener(&mut self, event: &str) {
        let id = self.id();
        if let NodeTypeMut::Element(mut element) = self.node_type_mut() {
            element.listeners_mut().remove(event);
        }
        self.dom.nodes_listening.get_mut(event).unwrap().remove(&id);
    }
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMut<'a, V> {
    pub fn node_type_mut(&mut self) -> NodeTypeMut<'_, V> {
        let Self { id, dom, dirty } = self;
        let node_type = dom.tree.get_mut::<NodeType<V>>(*id).unwrap();
        match node_type {
            NodeType::Element(element) => NodeTypeMut::Element(ElementNodeMut { element, dirty }),
            NodeType::Text(text) => {
                dirty.set_text();
                NodeTypeMut::Text(text)
            }
            NodeType::Placeholder => NodeTypeMut::Placeholder,
        }
    }

    pub fn set_type(&mut self, new: NodeType<V>) {
        *self.dom.tree.get_mut::<NodeType<V>>(self.id).unwrap() = new;
        self.dirty = NodeMaskBuilder::ALL.build();
    }
}

impl<V: FromAnyValue + Send + Sync> Drop for NodeMut<'_, V> {
    fn drop(&mut self) {
        let mask = std::mem::take(&mut self.dirty);
        self.dom.mark_dirty(self.id, mask);
    }
}

pub enum NodeTypeMut<'a, V: FromAnyValue = ()> {
    Element(ElementNodeMut<'a, V>),
    Text(&'a mut String),
    Placeholder,
}

pub struct ElementNodeMut<'a, V: FromAnyValue = ()> {
    element: &'a mut ElementNode<V>,
    dirty: &'a mut NodeMask,
}

impl<V: FromAnyValue> ElementNodeMut<'_, V> {
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

    pub fn attributes(&self) -> &FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>> {
        &self.element.attributes
    }

    pub fn attributes_mut(
        &mut self,
    ) -> &mut FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>> {
        self.dirty.add_attributes(AttributeMask::All);
        &mut self.element.attributes
    }

    pub fn set_attribute(
        &mut self,
        name: OwnedAttributeDiscription,
        value: OwnedAttributeValue<V>,
    ) -> Option<OwnedAttributeValue<V>> {
        self.dirty.add_attributes(AttributeMask::single(&name.name));
        self.element.attributes.insert(name, value)
    }

    pub fn remove_attributes(
        &mut self,
        name: &OwnedAttributeDiscription,
    ) -> Option<OwnedAttributeValue<V>> {
        self.dirty.add_attributes(AttributeMask::single(&name.name));
        self.element.attributes.remove(name)
    }

    pub fn get_attribute_mut(
        &mut self,
        name: &OwnedAttributeDiscription,
    ) -> Option<&mut OwnedAttributeValue<V>> {
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

pub struct NodeMutRaw<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    dom: &'a mut RealDom<V>,
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMutRaw<'a, V> {
    fn node_type_mut(&mut self) -> &mut NodeType<V> {
        self.dom.tree.get_mut::<NodeType<V>>(self.id).unwrap()
    }
}

impl<'a, V: FromAnyValue + Send + Sync> NodeImmutable<V> for NodeMutRaw<'a, V> {
    fn real_dom(&self) -> &RealDom<V> {
        self.dom
    }

    fn id(&self) -> NodeId {
        self.id
    }
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMutable<V> for NodeMutRaw<'a, V> {
    fn real_dom_mut(&mut self) -> &mut RealDom<V> {
        self.dom
    }

    fn add_event_listener(&mut self, event: &str) {
        let id = self.id();
        if let NodeType::Element(element) = self.node_type_mut() {
            element.listeners.insert(event.to_string());
            match self.dom.nodes_listening.get_mut(event) {
                Some(hs) => {
                    hs.insert(id);
                }
                None => {
                    let mut hs = FxHashSet::default();
                    hs.insert(id);
                    self.dom.nodes_listening.insert(event.to_string(), hs);
                }
            }
        }
    }

    fn remove_event_listener(&mut self, event: &str) {
        let id = self.id();
        if let NodeType::Element(element) = self.node_type_mut() {
            element.listeners.remove(event);
        }
        self.dom.nodes_listening.get_mut(event).unwrap().remove(&id);
    }
}

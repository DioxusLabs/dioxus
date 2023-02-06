use rustc_hash::{FxHashMap, FxHashSet};
use std::any::{Any, TypeId};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::RwLock;

use crate::node::{
    ElementNode, FromAnyValue, NodeType, OwnedAttributeDiscription, OwnedAttributeValue,
};
use crate::node_ref::{NodeMask, NodeMaskBuilder};
use crate::node_watcher::NodeWatcher;
use crate::passes::{resolve_passes, DirtyNodeStates, TypeErasedPass};
use crate::prelude::AttributeMaskBuilder;
use crate::tree::{NodeId, Tree};
use crate::{FxDashSet, SendAnyMap};

pub(crate) struct NodesDirty<V: FromAnyValue + Send + Sync> {
    passes_updated: FxHashMap<NodeId, FxHashSet<TypeId>>,
    nodes_updated: FxHashMap<NodeId, NodeMask>,
    pub(crate) passes: Box<[TypeErasedPass<V>]>,
}

impl<V: FromAnyValue + Send + Sync> NodesDirty<V> {
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
}

type NodeWatchers<V> = Rc<RwLock<Vec<Box<dyn NodeWatcher<V>>>>>;

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [crate::state::ParentDepState], [crate::state::NodeDepState], or [crate::state::ChildDepState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
///
/// # Custom values
/// To allow custom values to be passed into attributes implement FromAnyValue on a type that can represent your custom value and specify the V generic to be that type. If you have many different custom values, it can be useful to use a enum type to represent the varients.
pub struct RealDom<V: FromAnyValue + Send + Sync = ()> {
    pub(crate) tree: Tree,
    nodes_listening: FxHashMap<String, FxHashSet<NodeId>>,
    pub(crate) dirty_nodes: NodesDirty<V>,
    node_watchers: NodeWatchers<V>,
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
            dirty_nodes: NodesDirty {
                passes_updated,
                nodes_updated,
                passes,
            },
            node_watchers: Default::default(),
            phantom: std::marker::PhantomData,
        }
    }

    pub fn create_node(&mut self, node: NodeType<V>) -> NodeMut<'_, V> {
        let mut node_entry = self.tree.create_node();
        let id = node_entry.id();
        self.dirty_nodes
            .passes_updated
            .entry(id)
            .or_default()
            .extend(self.dirty_nodes.passes.iter().map(|x| x.this_type_id));
        node_entry.insert(node);
        let watchers = self.node_watchers.clone();
        for watcher in &*watchers.read().unwrap() {
            watcher.on_node_added(NodeMut::new(id, self));
        }
        NodeMut::new(id, self)
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
        let new_id = self.create_node(new_node).id();

        let children = self.tree.children_ids(node_id).unwrap().to_vec();
        for child in children {
            let child_id = self.clone_node(child);
            self.get_mut(new_id).unwrap().add_child(child_id);
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
    pub fn get_state_mut_raw<T: Any + Send + Sync>(&mut self, id: NodeId) -> Option<&mut T> {
        let id = id.into_node_id(self);
        self.tree.get_mut(id)
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state(
        &mut self,
        ctx: SendAnyMap,
        parallel: bool,
    ) -> (FxDashSet<NodeId>, FxHashMap<NodeId, NodeMask>) {
        let passes = std::mem::take(&mut self.dirty_nodes.passes_updated);
        let nodes_updated = std::mem::take(&mut self.dirty_nodes.nodes_updated);
        let dirty_nodes =
            DirtyNodeStates::with_passes(self.dirty_nodes.passes.iter().map(|p| p.this_type_id));
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

    pub fn add_node_watcher(&mut self, watcher: impl NodeWatcher<V> + 'static) {
        self.node_watchers.write().unwrap().push(Box::new(watcher));
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

    fn next(&self) -> Option<NodeRef<V>> {
        let parent = self.parent_id()?;
        let children = self.real_dom().tree.children_ids(parent)?;
        let index = children.iter().position(|id| *id == self.id())?;
        if index + 1 < children.len() {
            Some(NodeRef {
                id: children[index + 1],
                dom: self.real_dom(),
            })
        } else {
            None
        }
    }

    fn prev(&self) -> Option<NodeRef<V>> {
        let parent = self.parent_id()?;
        let children = self.real_dom().tree.children_ids(parent)?;
        let index = children.iter().position(|id| *id == self.id())?;
        if index > 0 {
            Some(NodeRef {
                id: children[index - 1],
                dom: self.real_dom(),
            })
        } else {
            None
        }
    }
}

pub struct NodeRef<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    dom: &'a RealDom<V>,
}

impl<'a, V: FromAnyValue + Send + Sync> Clone for NodeRef<'a, V> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            dom: self.dom,
        }
    }
}

impl<'a, V: FromAnyValue + Send + Sync> Copy for NodeRef<'a, V> {}

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
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMut<'a, V> {
    pub fn new(id: NodeId, dom: &'a mut RealDom<V>) -> Self {
        Self { id, dom }
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

impl<'a, V: FromAnyValue + Send + Sync> NodeMut<'a, V> {
    pub fn real_dom_mut(&mut self) -> &mut RealDom<V> {
        self.dom
    }

    pub fn get_mut<T: Any + Sync + Send>(&mut self) -> Option<&mut T> {
        // mark the node state as dirty
        self.dom
            .dirty_nodes
            .passes_updated
            .entry(self.id)
            .or_default()
            .insert(TypeId::of::<T>());
        self.dom.tree.get_mut(self.id)
    }

    pub fn insert<T: Any + Sync + Send>(&mut self, value: T) {
        // mark the node state as dirty
        self.dom
            .dirty_nodes
            .passes_updated
            .entry(self.id)
            .or_default()
            .insert(TypeId::of::<T>());
        self.dom.tree.insert(self.id, value);
    }

    pub fn next_mut(self) -> Option<NodeMut<'a, V>> {
        let parent = self.parent_id()?;
        let children = self.dom.tree.children_ids(parent)?;
        let index = children.iter().position(|id| *id == self.id)?;
        if index + 1 < children.len() {
            Some(NodeMut::new(children[index + 1], self.dom))
        } else {
            None
        }
    }

    pub fn prev_mut(self) -> Option<NodeMut<'a, V>> {
        let parent = self.parent_id()?;
        let children = self.dom.tree.children_ids(parent)?;
        let index = children.iter().position(|id| *id == self.id)?;
        if index > 0 {
            Some(NodeMut::new(children[index - 1], self.dom))
        } else {
            None
        }
    }

    pub fn add_child(&mut self, child: NodeId) {
        self.dom.dirty_nodes.mark_child_changed(self.id);
        self.dom.dirty_nodes.mark_parent_added_or_removed(child);
        self.dom.tree.add_child(self.id, child);
        NodeMut::new(child, self.dom).mark_moved();
    }

    pub fn insert_after(&mut self, old: NodeId) {
        let id = self.id();
        if let Some(parent_id) = self.dom.tree.parent_id(old) {
            self.dom.dirty_nodes.mark_child_changed(parent_id);
            self.dom.dirty_nodes.mark_parent_added_or_removed(id);
        }
        self.dom.tree.insert_after(old, id);
        self.mark_moved();
    }

    pub fn insert_before(&mut self, old: NodeId) {
        let id = self.id();
        if let Some(parent_id) = self.dom.tree.parent_id(old) {
            self.dom.dirty_nodes.mark_child_changed(parent_id);
            self.dom.dirty_nodes.mark_parent_added_or_removed(id);
        }
        self.dom.tree.insert_before(old, id);
        self.mark_moved();
    }

    pub fn remove(&mut self) {
        let id = self.id();
        self.mark_removed();
        if let Some(parent_id) = self.real_dom_mut().tree.parent_id(id) {
            self.real_dom_mut()
                .dirty_nodes
                .mark_child_changed(parent_id);
        }
        if let Some(children_ids) = self.child_ids() {
            let children_ids_vec = children_ids.to_vec();
            for child in children_ids_vec {
                self.dom.get_mut(child).unwrap().remove();
            }
        }
        self.dom.tree.remove_single(id);
    }

    pub fn replace(&mut self, new: NodeId) {
        self.mark_removed();
        if let Some(parent_id) = self.parent_id() {
            self.real_dom_mut()
                .dirty_nodes
                .mark_child_changed(parent_id);
            self.real_dom_mut()
                .dirty_nodes
                .mark_parent_added_or_removed(new);
        }
        let id = self.id();
        self.dom.tree.replace(id, new);
    }

    pub fn add_event_listener(&mut self, event: &str) {
        let id = self.id();
        let node_type: &mut NodeType<V> = self.dom.tree.get_mut(self.id).unwrap();
        if let NodeType::Element(ElementNode { listeners, .. }) = node_type {
            self.dom
                .dirty_nodes
                .mark_dirty(self.id, NodeMaskBuilder::new().with_listeners().build());
            listeners.insert(event.to_string());
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

    pub fn remove_event_listener(&mut self, event: &str) {
        let id = self.id();
        let node_type: &mut NodeType<V> = self.dom.tree.get_mut(self.id).unwrap();
        if let NodeType::Element(ElementNode { listeners, .. }) = node_type {
            self.dom
                .dirty_nodes
                .mark_dirty(self.id, NodeMaskBuilder::new().with_listeners().build());
            listeners.remove(event);

            self.dom.nodes_listening.get_mut(event).unwrap().remove(&id);
        }
    }

    fn mark_removed(&mut self) {
        let watchers = self.dom.node_watchers.clone();
        for watcher in &*watchers.read().unwrap() {
            watcher.on_node_removed(NodeMut::new(self.id(), self.dom));
        }
    }

    fn mark_moved(&mut self) {
        let watchers = self.dom.node_watchers.clone();
        for watcher in &*watchers.read().unwrap() {
            watcher.on_node_moved(NodeMut::new(self.id(), self.dom));
        }
    }

    pub fn node_type_mut(&mut self) -> NodeTypeMut<'_, V> {
        let Self { id, dom } = self;
        let RealDom {
            dirty_nodes, tree, ..
        } = dom;
        let node_type = tree.get_mut(*id).unwrap();
        match node_type {
            NodeType::Element(element) => NodeTypeMut::Element(ElementNodeMut {
                id: *id,
                element,
                dirty_nodes,
            }),
            NodeType::Text(text) => {
                dirty_nodes.mark_dirty(self.id, NodeMaskBuilder::new().with_text().build());

                NodeTypeMut::Text(text)
            }
            NodeType::Placeholder => NodeTypeMut::Placeholder,
        }
    }

    pub fn set_type(&mut self, new: NodeType<V>) {
        *self.dom.tree.get_mut::<NodeType<V>>(self.id).unwrap() = new;
        self.dom
            .dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::ALL.build())
    }
}

pub enum NodeTypeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    Element(ElementNodeMut<'a, V>),
    Text(&'a mut String),
    Placeholder,
}

pub struct ElementNodeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    element: &'a mut ElementNode<V>,
    dirty_nodes: &'a mut NodesDirty<V>,
}

impl<V: FromAnyValue + Send + Sync> ElementNodeMut<'_, V> {
    pub fn tag(&self) -> &str {
        &self.element.tag
    }

    pub fn tag_mut(&mut self) -> &mut String {
        self.dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::new().with_tag().build());
        &mut self.element.tag
    }

    pub fn namespace(&self) -> Option<&str> {
        self.element.namespace.as_deref()
    }

    pub fn namespace_mut(&mut self) -> &mut Option<String> {
        self.dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::new().with_namespace().build());
        &mut self.element.namespace
    }

    pub fn attributes(&self) -> &FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>> {
        &self.element.attributes
    }

    pub fn set_attribute(
        &mut self,
        name: OwnedAttributeDiscription,
        value: OwnedAttributeValue<V>,
    ) -> Option<OwnedAttributeValue<V>> {
        self.dirty_nodes.mark_dirty(
            self.id,
            NodeMaskBuilder::new()
                .with_attrs(AttributeMaskBuilder::Some(&[&name.name]))
                .build(),
        );
        self.element.attributes.insert(name, value)
    }

    pub fn remove_attributes(
        &mut self,
        name: &OwnedAttributeDiscription,
    ) -> Option<OwnedAttributeValue<V>> {
        self.dirty_nodes.mark_dirty(
            self.id,
            NodeMaskBuilder::new()
                .with_attrs(AttributeMaskBuilder::Some(&[&name.name]))
                .build(),
        );
        self.element.attributes.remove(name)
    }

    pub fn get_attribute_mut(
        &mut self,
        name: &OwnedAttributeDiscription,
    ) -> Option<&mut OwnedAttributeValue<V>> {
        self.dirty_nodes.mark_dirty(
            self.id,
            NodeMaskBuilder::new()
                .with_attrs(AttributeMaskBuilder::Some(&[&name.name]))
                .build(),
        );
        self.element.attributes.get_mut(name)
    }

    pub fn listeners(&self) -> &FxHashSet<String> {
        &self.element.listeners
    }
}

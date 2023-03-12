use rustc_hash::{FxHashMap, FxHashSet};
use shipyard::error::GetStorage;
use shipyard::track::Untracked;
use shipyard::{Component, Get, IntoBorrow, ScheduledWorkload, Unique, View, ViewMut, Workload};
use shipyard::{SystemModificator, World};
use std::any::TypeId;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use crate::node::{
    ElementNode, FromAnyValue, NodeType, OwnedAttributeDiscription, OwnedAttributeValue, TextNode,
};
use crate::node_ref::{NodeMask, NodeMaskBuilder};
use crate::node_watcher::NodeWatcher;
use crate::passes::{DirtyNodeStates, TypeErasedPass};
use crate::prelude::AttributeMaskBuilder;
use crate::tree::{TreeMut, TreeMutView, TreeRef, TreeRefView};
use crate::NodeId;
use crate::{FxDashSet, SendAnyMap};

#[derive(Unique)]
pub struct SendAnyMapWrapper(SendAnyMap);

impl Deref for SendAnyMapWrapper {
    type Target = SendAnyMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Unique, Default)]
pub struct DirtyNodesResult(FxDashSet<NodeId>);

impl Deref for DirtyNodesResult {
    type Target = FxDashSet<NodeId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct NodesDirty<V: FromAnyValue + Send + Sync> {
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

type NodeWatchers<V> = Arc<RwLock<Vec<Box<dyn NodeWatcher<V> + Send + Sync>>>>;

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [crate::state::ParentDepState], [crate::state::NodeDepState], or [crate::state::ChildDepState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
///
/// # Custom values
/// To allow custom values to be passed into attributes implement FromAnyValue on a type that can represent your custom value and specify the V generic to be that type. If you have many different custom values, it can be useful to use a enum type to represent the varients.
pub struct RealDom<V: FromAnyValue + Send + Sync = ()> {
    pub(crate) world: World,
    nodes_listening: FxHashMap<String, FxHashSet<NodeId>>,
    pub(crate) dirty_nodes: NodesDirty<V>,
    node_watchers: NodeWatchers<V>,
    workload: ScheduledWorkload,
    root_id: NodeId,
    phantom: std::marker::PhantomData<V>,
}

impl<V: FromAnyValue + Send + Sync> RealDom<V> {
    pub fn new(mut passes: Box<[TypeErasedPass<V>]>) -> RealDom<V> {
        // resolve dependants for each pass
        for i in 1..passes.len() {
            let (before, after) = passes.split_at_mut(i);
            let (current, before) = before.split_last_mut().unwrap();
            for pass in before.iter_mut().chain(after.iter_mut()) {
                if current
                    .combined_dependancy_type_ids
                    .contains(&pass.this_type_id)
                {
                    pass.dependants.insert(current.this_type_id);
                }
            }
        }
        let workload = construct_workload(&mut passes);
        let (workload, _) = workload.build().unwrap();
        let mut world = World::new();
        let root_node: NodeType<V> = NodeType::Element(ElementNode {
            tag: "Root".to_string(),
            namespace: Some("Root".to_string()),
            attributes: FxHashMap::default(),
            listeners: FxHashSet::default(),
        });
        let root_id = world.add_entity(root_node);
        {
            let mut tree: TreeMutView = world.borrow().unwrap();
            tree.create_node(root_id);
        }

        let mut passes_updated = FxHashMap::default();
        let mut nodes_updated = FxHashMap::default();

        passes_updated.insert(root_id, passes.iter().map(|x| x.this_type_id).collect());
        nodes_updated.insert(root_id, NodeMaskBuilder::ALL.build());

        RealDom {
            world,
            nodes_listening: FxHashMap::default(),
            dirty_nodes: NodesDirty {
                passes_updated,
                nodes_updated,
                passes,
            },
            node_watchers: Default::default(),
            workload,
            root_id,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn tree_mut(&self) -> TreeMutView {
        self.world.borrow().unwrap()
    }

    pub fn tree_ref(&self) -> TreeRefView {
        self.world.borrow().unwrap()
    }

    pub fn create_node(&mut self, node: NodeType<V>) -> NodeMut<'_, V> {
        let id = self.world.add_entity(node);
        self.tree_mut().create_node(id);
        self.dirty_nodes
            .passes_updated
            .entry(id)
            .or_default()
            .extend(self.dirty_nodes.passes.iter().map(|x| x.this_type_id));
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
                .map(|id| (*id, self.tree_ref().height(*id).unwrap()))
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

    /// Returns the id of the root node.
    pub fn root_id(&self) -> NodeId {
        self.root_id
    }

    pub fn get(&self, id: NodeId) -> Option<NodeRef<'_, V>> {
        self.tree_ref()
            .contains(id)
            .then_some(NodeRef { id, dom: self })
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<NodeMut<'_, V>> {
        let contains = self.tree_ref().contains(id);
        contains.then(|| NodeMut::new(id, self))
    }

    fn borrow_raw<'a, B: IntoBorrow>(&'a self) -> Result<B, GetStorage>
    where
        B::Borrow: shipyard::Borrow<'a, View = B>,
    {
        self.world.borrow()
    }

    fn borrow_node_type_mut(&self) -> Result<ViewMut<NodeType<V>>, GetStorage> {
        self.world.borrow()
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state(
        &mut self,
        ctx: SendAnyMap,
    ) -> (FxDashSet<NodeId>, FxHashMap<NodeId, NodeMask>) {
        let passes = std::mem::take(&mut self.dirty_nodes.passes_updated);
        let nodes_updated = std::mem::take(&mut self.dirty_nodes.nodes_updated);
        let dirty_nodes =
            DirtyNodeStates::with_passes(self.dirty_nodes.passes.iter().map(|p| p.this_type_id));
        let tree = self.tree_ref();
        for (node_id, passes) in passes {
            // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
            if let Some(height) = tree.height(node_id) {
                for pass in passes {
                    dirty_nodes.insert(pass, node_id, height);
                }
            }
        }

        let _ = self.world.remove_unique::<DirtyNodeStates>();
        let _ = self.world.remove_unique::<SendAnyMapWrapper>();
        self.world.add_unique(dirty_nodes);
        self.world.add_unique(SendAnyMapWrapper(ctx));
        self.world.add_unique(DirtyNodesResult::default());

        self.workload.run_with_world(&self.world).unwrap();

        let dirty = self.world.remove_unique::<DirtyNodesResult>().unwrap();

        (dirty.0, nodes_updated)
    }

    pub fn traverse_depth_first(&self, mut f: impl FnMut(NodeRef<V>)) {
        let mut stack = vec![self.root_id()];
        let tree = self.tree_ref();
        while let Some(id) = stack.pop() {
            if let Some(node) = self.get(id) {
                f(node);
                let children = tree.children_ids(id);
                stack.extend(children.iter().copied().rev());
            }
        }
    }

    pub fn traverse_breadth_first(&self, mut f: impl FnMut(NodeRef<V>)) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root_id());
        let tree = self.tree_ref();
        while let Some(id) = queue.pop_front() {
            if let Some(node) = self.get(id) {
                f(node);
                let children = tree.children_ids(id);
                for id in children {
                    queue.push_back(id);
                }
            }
        }
    }

    pub fn traverse_depth_first_mut(&mut self, mut f: impl FnMut(NodeMut<V>)) {
        let mut stack = vec![self.root_id()];
        while let Some(id) = stack.pop() {
            let tree = self.tree_ref();
            let mut children = tree.children_ids(id);
            drop(tree);
            children.reverse();
            if let Some(node) = self.get_mut(id) {
                let node = node;
                f(node);
                stack.extend(children.iter());
            }
        }
    }

    pub fn traverse_breadth_first_mut(&mut self, mut f: impl FnMut(NodeMut<V>)) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root_id());
        while let Some(id) = queue.pop_front() {
            let tree = self.tree_ref();
            let children = tree.children_ids(id);
            drop(tree);
            if let Some(node) = self.get_mut(id) {
                f(node);
                for id in children {
                    queue.push_back(id);
                }
            }
        }
    }

    pub fn add_node_watcher(&mut self, watcher: impl NodeWatcher<V> + 'static + Send + Sync) {
        self.node_watchers.write().unwrap().push(Box::new(watcher));
    }

    /// Returns a reference to the underlying world. Any changes made to the world will not update the reactive system.
    pub fn raw_world(&self) -> &World {
        &self.world
    }

    /// Returns a mutable reference to the underlying world. Any changes made to the world will not update the reactive system.
    pub fn raw_world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

pub struct ViewEntry<'a, V: Component + Send + Sync> {
    view: View<'a, V>,
    id: NodeId,
}

impl<'a, V: Component + Send + Sync> ViewEntry<'a, V> {
    fn new(view: View<'a, V>, id: NodeId) -> Self {
        Self { view, id }
    }
}

impl<'a, V: Component + Send + Sync> Deref for ViewEntry<'a, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.view[self.id]
    }
}

pub struct ViewEntryMut<'a, V: Component<Tracking = Untracked> + Send + Sync> {
    view: ViewMut<'a, V, Untracked>,
    id: NodeId,
}

impl<'a, V: Component<Tracking = Untracked> + Send + Sync> ViewEntryMut<'a, V> {
    fn new(view: ViewMut<'a, V, Untracked>, id: NodeId) -> Self {
        Self { view, id }
    }
}

impl<'a, V: Component<Tracking = Untracked> + Send + Sync> Deref for ViewEntryMut<'a, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.view.get(self.id).unwrap()
    }
}

impl<'a, V: Component<Tracking = Untracked> + Send + Sync> DerefMut for ViewEntryMut<'a, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        (&mut self.view).get(self.id).unwrap()
    }
}

pub trait NodeImmutable<V: FromAnyValue + Send + Sync>: Sized {
    fn real_dom(&self) -> &RealDom<V>;

    fn id(&self) -> NodeId;

    #[inline]
    fn node_type(&self) -> ViewEntry<NodeType<V>> {
        self.get().unwrap()
    }

    #[inline]
    fn get<'a, T: Component + Sync + Send>(&'a self) -> Option<ViewEntry<'a, T>> {
        // self.real_dom().tree.get(self.id())
        let view: View<'a, T> = self.real_dom().borrow_raw().ok()?;
        view.contains(self.id())
            .then(|| ViewEntry::new(view, self.id()))
    }

    #[inline]
    fn child_ids(&self) -> Vec<NodeId> {
        self.real_dom().tree_ref().children_ids(self.id())
    }

    #[inline]
    fn children(&self) -> Vec<NodeRef<V>> {
        self.child_ids()
            .iter()
            .map(|id| NodeRef {
                id: *id,
                dom: self.real_dom(),
            })
            .collect()
    }

    #[inline]
    fn parent_id(&self) -> Option<NodeId> {
        self.real_dom().tree_ref().parent_id(self.id())
    }

    #[inline]
    fn parent(&self) -> Option<NodeRef<V>> {
        self.parent_id().map(|id| NodeRef {
            id,
            dom: self.real_dom(),
        })
    }

    #[inline]
    fn next(&self) -> Option<NodeRef<V>> {
        let parent = self.parent_id()?;
        let children = self.real_dom().tree_ref().children_ids(parent);
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

    #[inline]
    fn prev(&self) -> Option<NodeRef<V>> {
        let parent = self.parent_id()?;
        let children = self.real_dom().tree_ref().children_ids(parent);
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

    #[inline]
    fn height(&self) -> u16 {
        self.real_dom().tree_ref().height(self.id()).unwrap()
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
    #[inline(always)]
    fn real_dom(&self) -> &RealDom<V> {
        self.dom
    }

    #[inline(always)]
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
    #[inline(always)]
    fn real_dom(&self) -> &RealDom<V> {
        self.dom
    }

    #[inline(always)]
    fn id(&self) -> NodeId {
        self.id
    }
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMut<'a, V> {
    #[inline(always)]
    pub fn real_dom_mut(&mut self) -> &mut RealDom<V> {
        self.dom
    }

    #[inline]
    pub fn parent_mut(&mut self) -> Option<NodeMut<V>> {
        self.parent_id().map(|id| NodeMut { id, dom: self.dom })
    }

    #[inline]
    pub fn get_mut<T: Component<Tracking = Untracked> + Sync + Send>(
        &mut self,
    ) -> Option<ViewEntryMut<T>> {
        // mark the node state as dirty
        self.dom
            .dirty_nodes
            .passes_updated
            .entry(self.id)
            .or_default()
            .insert(TypeId::of::<T>());
        let view_mut: ViewMut<T> = self.dom.borrow_raw().ok()?;
        Some(ViewEntryMut::new(view_mut, self.id))
    }

    #[inline]
    pub fn insert<T: Component + Sync + Send>(&mut self, value: T) {
        // mark the node state as dirty
        self.dom
            .dirty_nodes
            .passes_updated
            .entry(self.id)
            .or_default()
            .insert(TypeId::of::<T>());
        self.dom.world.add_component(self.id, value);
    }

    #[inline]
    pub fn next_mut(self) -> Option<NodeMut<'a, V>> {
        let parent = self.parent_id()?;
        let children = self.dom.tree_mut().children_ids(parent);
        let index = children.iter().position(|id| *id == self.id)?;
        if index + 1 < children.len() {
            Some(NodeMut::new(children[index + 1], self.dom))
        } else {
            None
        }
    }

    #[inline]
    pub fn prev_mut(self) -> Option<NodeMut<'a, V>> {
        let parent = self.parent_id()?;
        let children = self.dom.tree_ref().children_ids(parent);
        let index = children.iter().position(|id| *id == self.id)?;
        if index > 0 {
            Some(NodeMut::new(children[index - 1], self.dom))
        } else {
            None
        }
    }

    #[inline]
    pub fn add_child(&mut self, child: NodeId) {
        self.dom.dirty_nodes.mark_child_changed(self.id);
        self.dom.dirty_nodes.mark_parent_added_or_removed(child);
        self.dom.tree_mut().add_child(self.id, child);
        NodeMut::new(child, self.dom).mark_moved();
    }

    #[inline]
    pub fn insert_after(&mut self, old: NodeId) {
        let id = self.id();
        let parent_id = { self.dom.tree_ref().parent_id(old) };
        if let Some(parent_id) = parent_id {
            self.dom.dirty_nodes.mark_child_changed(parent_id);
            self.dom.dirty_nodes.mark_parent_added_or_removed(id);
        }
        self.dom.tree_mut().insert_after(old, id);
        self.mark_moved();
    }

    #[inline]
    pub fn insert_before(&mut self, old: NodeId) {
        let id = self.id();
        let parent_id = { self.dom.tree_ref().parent_id(old) };
        if let Some(parent_id) = parent_id {
            self.dom.dirty_nodes.mark_child_changed(parent_id);
            self.dom.dirty_nodes.mark_parent_added_or_removed(id);
        }
        self.dom.tree_mut().insert_before(old, id);
        self.mark_moved();
    }

    #[inline]
    pub fn remove(&mut self) {
        let id = self.id();
        {
            let RealDom {
                world,
                nodes_listening,
                ..
            } = &mut self.dom;
            let mut view: ViewMut<NodeType<V>> = world.borrow().unwrap();
            if let NodeType::Element(ElementNode { listeners, .. })
            | NodeType::Text(TextNode { listeners, .. }) = (&mut view).get(id).unwrap()
            {
                let listeners = std::mem::take(listeners);
                for event in listeners {
                    nodes_listening.get_mut(&event).unwrap().remove(&id);
                }
            }
        }
        self.mark_removed();
        let parent_id = { self.dom.tree_ref().parent_id(id) };
        if let Some(parent_id) = parent_id {
            self.real_dom_mut()
                .dirty_nodes
                .mark_child_changed(parent_id);
        }
        let children_ids = self.child_ids();
        let children_ids_vec = children_ids.to_vec();
        for child in children_ids_vec {
            self.dom.get_mut(child).unwrap().remove();
        }
        self.dom.tree_mut().remove_single(id);
    }

    #[inline]
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
        self.dom.tree_mut().replace(id, new);
    }

    #[inline]
    pub fn add_event_listener(&mut self, event: &str) {
        let id = self.id();
        let RealDom {
            world,
            dirty_nodes,
            nodes_listening,
            ..
        } = &mut self.dom;
        let mut view: ViewMut<NodeType<V>> = world.borrow().unwrap();
        let node_type: &mut NodeType<V> = (&mut view).get(self.id).unwrap();
        if let NodeType::Element(ElementNode { listeners, .. })
        | NodeType::Text(TextNode { listeners, .. }) = node_type
        {
            dirty_nodes.mark_dirty(self.id, NodeMaskBuilder::new().with_listeners().build());
            listeners.insert(event.to_string());
            match nodes_listening.get_mut(event) {
                Some(hs) => {
                    hs.insert(id);
                }
                None => {
                    let mut hs = FxHashSet::default();
                    hs.insert(id);
                    nodes_listening.insert(event.to_string(), hs);
                }
            }
        }
    }

    #[inline]
    pub fn remove_event_listener(&mut self, event: &str) {
        let id = self.id();
        let RealDom {
            world,
            dirty_nodes,
            nodes_listening,
            ..
        } = &mut self.dom;
        let mut view: ViewMut<NodeType<V>> = world.borrow().unwrap();
        let node_type: &mut NodeType<V> = (&mut view).get(self.id).unwrap();
        if let NodeType::Element(ElementNode { listeners, .. })
        | NodeType::Text(TextNode { listeners, .. }) = node_type
        {
            dirty_nodes.mark_dirty(self.id, NodeMaskBuilder::new().with_listeners().build());
            listeners.remove(event);

            nodes_listening.get_mut(event).unwrap().remove(&id);
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
        let id = self.id();
        let RealDom {
            world, dirty_nodes, ..
        } = &mut self.dom;
        let view: ViewMut<NodeType<V>> = world.borrow().unwrap();
        let node_type = ViewEntryMut::new(view, id);
        match &*node_type {
            NodeType::Element(_) => NodeTypeMut::Element(ElementNodeMut {
                id,
                element: node_type,
                dirty_nodes,
            }),
            NodeType::Text(_) => NodeTypeMut::Text(TextNodeMut {
                id,
                text: node_type,
                dirty_nodes,
            }),
            NodeType::Placeholder => NodeTypeMut::Placeholder,
        }
    }

    pub fn set_type(&mut self, new: NodeType<V>) {
        {
            let mut view: ViewMut<NodeType<V>> = self.dom.borrow_node_type_mut().unwrap();
            *(&mut view).get(self.id).unwrap() = new;
        }
        self.dom
            .dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::ALL.build())
    }

    #[inline]
    pub fn clone_node(&mut self) -> NodeId {
        let new_node = self.node_type().clone();
        let rdom = self.real_dom_mut();
        let new_id = rdom.create_node(new_node).id();

        let children = self.child_ids();
        let children = children.to_vec();
        let rdom = self.real_dom_mut();
        for child in children {
            let child_id = rdom.get_mut(child).unwrap().clone_node();
            rdom.get_mut(new_id).unwrap().add_child(child_id);
        }
        new_id
    }
}

pub enum NodeTypeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    Element(ElementNodeMut<'a, V>),
    Text(TextNodeMut<'a, V>),
    Placeholder,
}

pub struct TextNodeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    text: ViewEntryMut<'a, NodeType<V>>,
    dirty_nodes: &'a mut NodesDirty<V>,
}

impl<V: FromAnyValue + Send + Sync> TextNodeMut<'_, V> {
    pub fn text(&self) -> &str {
        match &*self.text {
            NodeType::Text(text) => &text.text,
            _ => unreachable!(),
        }
    }

    pub fn text_mut(&mut self) -> &mut String {
        self.dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::new().with_text().build());
        match &mut *self.text {
            NodeType::Text(text) => &mut text.text,
            _ => unreachable!(),
        }
    }
}

impl<V: FromAnyValue + Send + Sync> Deref for TextNodeMut<'_, V> {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        match &*self.text {
            NodeType::Text(text) => &text.text,
            _ => unreachable!(),
        }
    }
}

impl<V: FromAnyValue + Send + Sync> DerefMut for TextNodeMut<'_, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.text_mut()
    }
}

pub struct ElementNodeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    element: ViewEntryMut<'a, NodeType<V>>,
    dirty_nodes: &'a mut NodesDirty<V>,
}

impl<V: FromAnyValue + Send + Sync> ElementNodeMut<'_, V> {
    fn element(&self) -> &ElementNode<V> {
        match &*self.element {
            NodeType::Element(element) => element,
            _ => unreachable!(),
        }
    }

    fn element_mut(&mut self) -> &mut ElementNode<V> {
        match &mut *self.element {
            NodeType::Element(element) => element,
            _ => unreachable!(),
        }
    }

    pub fn tag(&self) -> &str {
        &self.element().tag
    }

    pub fn tag_mut(&mut self) -> &mut String {
        self.dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::new().with_tag().build());
        &mut self.element_mut().tag
    }

    pub fn namespace(&self) -> Option<&str> {
        self.element().namespace.as_deref()
    }

    pub fn namespace_mut(&mut self) -> &mut Option<String> {
        self.dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::new().with_namespace().build());
        &mut self.element_mut().namespace
    }

    pub fn attributes(&self) -> &FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>> {
        &self.element().attributes
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
        self.element_mut().attributes.insert(name, value)
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
        self.element_mut().attributes.remove(name)
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
        self.element_mut().attributes.get_mut(name)
    }

    pub fn listeners(&self) -> &FxHashSet<String> {
        &self.element().listeners
    }
}

fn construct_workload<V: FromAnyValue + Send + Sync>(passes: &mut [TypeErasedPass<V>]) -> Workload {
    let mut workload = Workload::new("Main Workload");
    let mut unresloved_workloads = passes
        .iter_mut()
        .enumerate()
        .map(|(i, pass)| {
            let workload = Some(pass.create_workload());
            (i, pass, workload)
        })
        .collect::<Vec<_>>();
    // set all the labels
    for (id, _, workload) in &mut unresloved_workloads {
        *workload = Some(workload.take().unwrap().tag(id.to_string()));
    }
    // mark any dependancies
    for i in 0..unresloved_workloads.len() {
        let (_, pass, _) = &unresloved_workloads[i];
        for ty_id in pass.combined_dependancy_type_ids.clone() {
            let &(dependancy_id, _, _) = unresloved_workloads
                .iter()
                .find(|(_, pass, _)| pass.this_type_id == ty_id)
                .unwrap();
            let (_, _, workload) = &mut unresloved_workloads[i];
            *workload = workload
                .take()
                .map(|workload| workload.after_all(dependancy_id.to_string()));
        }
    }
    for (_, _, mut workload_system) in unresloved_workloads {
        workload = workload.with_system(workload_system.take().unwrap());
    }
    workload
}

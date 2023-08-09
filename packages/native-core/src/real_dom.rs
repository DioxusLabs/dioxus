//! A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.

use rustc_hash::{FxHashMap, FxHashSet};
use shipyard::error::GetStorage;
use shipyard::track::Untracked;
use shipyard::{Component, Get, IntoBorrow, ScheduledWorkload, Unique, View, ViewMut, Workload};
use shipyard::{SystemModificator, World};
use std::any::TypeId;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use crate::custom_element::{
    CustomElement, CustomElementFactory, CustomElementManager, CustomElementRegistry,
    CustomElementUpdater,
};
use crate::node::{
    ElementNode, FromAnyValue, NodeType, OwnedAttributeDiscription, OwnedAttributeValue, TextNode,
};
use crate::node_ref::{NodeMask, NodeMaskBuilder};
use crate::node_watcher::{AttributeWatcher, NodeWatcher};
use crate::passes::{Dependant, DirtyNodeStates, PassDirection, TypeErasedState};
use crate::prelude::AttributeMaskBuilder;
use crate::tree::{TreeMut, TreeMutView, TreeRef, TreeRefView};
use crate::NodeId;
use crate::{FxDashSet, SendAnyMap};

/// The context passes can receive when they are executed
#[derive(Unique)]
pub(crate) struct SendAnyMapWrapper(SendAnyMap);

impl Deref for SendAnyMapWrapper {
    type Target = SendAnyMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The nodes that were changed when updating the state of the RealDom
#[derive(Unique, Default)]
pub(crate) struct DirtyNodesResult(FxDashSet<NodeId>);

impl Deref for DirtyNodesResult {
    type Target = FxDashSet<NodeId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The nodes that have been marked as dirty in the RealDom
pub(crate) struct NodesDirty<V: FromAnyValue + Send + Sync> {
    passes_updated: FxHashMap<NodeId, FxHashSet<TypeId>>,
    nodes_updated: FxHashMap<NodeId, NodeMask>,
    nodes_created: FxHashSet<NodeId>,
    pub(crate) passes: Box<[TypeErasedState<V>]>,
}

impl<V: FromAnyValue + Send + Sync> NodesDirty<V> {
    /// Mark a node as dirty
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

    /// Mark a node that has had a parent changed
    fn mark_parent_added_or_removed(&mut self, node_id: NodeId) {
        let hm = self.passes_updated.entry(node_id).or_default();
        for pass in &*self.passes {
            // If any of the states in this node depend on the parent then mark them as dirty
            for &pass in &pass.parent_dependancies_ids {
                hm.insert(pass);
            }
        }
    }

    /// Mark a node as having a child added or removed
    fn mark_child_changed(&mut self, node_id: NodeId) {
        let hm = self.passes_updated.entry(node_id).or_default();
        for pass in &*self.passes {
            // If any of the states in this node depend on the children then mark them as dirty
            for &pass in &pass.child_dependancies_ids {
                hm.insert(pass);
            }
        }
    }
}

type NodeWatchers<V> = Arc<RwLock<Vec<Box<dyn NodeWatcher<V> + Send + Sync>>>>;
type AttributeWatchers<V> = Arc<RwLock<Vec<Box<dyn AttributeWatcher<V> + Send + Sync>>>>;

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started:
/// 1) Implement [crate::passes::State] for each part of your state that you want to compute incrementally
/// 2) Create a RealDom [RealDom::new], passing in each state you created
/// 3) Update the state of the RealDom by adding and modifying nodes
/// 4) Call [RealDom::update_state] to update the state of incrementally computed values on each node
///
/// # Custom attribute values
/// To allow custom values to be passed into attributes implement FromAnyValue on a type that can represent your custom value and specify the V generic to be that type. If you have many different custom values, it can be useful to use a enum type to represent the varients.
pub struct RealDom<V: FromAnyValue + Send + Sync = ()> {
    pub(crate) world: World,
    nodes_listening: FxHashMap<String, FxHashSet<NodeId>>,
    pub(crate) dirty_nodes: NodesDirty<V>,
    node_watchers: NodeWatchers<V>,
    attribute_watchers: AttributeWatchers<V>,
    workload: ScheduledWorkload,
    root_id: NodeId,
    custom_elements: Arc<RwLock<CustomElementRegistry<V>>>,
    phantom: std::marker::PhantomData<V>,
}

impl<V: FromAnyValue + Send + Sync> RealDom<V> {
    /// Create a new RealDom with the given states that will be inserted and updated when needed
    pub fn new(tracked_states: impl Into<Box<[TypeErasedState<V>]>>) -> RealDom<V> {
        let mut tracked_states = tracked_states.into();
        // resolve dependants for each pass
        for i in 1..=tracked_states.len() {
            let (before, after) = tracked_states.split_at_mut(i);
            let (current, before) = before.split_last_mut().unwrap();
            for state in before.iter_mut().chain(after.iter_mut()) {
                let dependants = Arc::get_mut(&mut state.dependants).unwrap();

                let current_dependant = Dependant {
                    type_id: current.this_type_id,
                    enter_shadow_dom: current.enter_shadow_dom,
                };

                // If this node depends on the other state as a parent, then the other state should update its children of the current type when it is invalidated
                if current
                    .parent_dependancies_ids
                    .contains(&state.this_type_id)
                    && !dependants.child.contains(&current_dependant)
                {
                    dependants.child.push(current_dependant);
                }
                // If this node depends on the other state as a child, then the other state should update its parent of the current type when it is invalidated
                if current.child_dependancies_ids.contains(&state.this_type_id)
                    && !dependants.parent.contains(&current_dependant)
                {
                    dependants.parent.push(current_dependant);
                }
                // If this node depends on the other state as a sibling, then the other state should update its siblings of the current type when it is invalidated
                if current.node_dependancies_ids.contains(&state.this_type_id)
                    && !dependants.node.contains(&current.this_type_id)
                {
                    dependants.node.push(current.this_type_id);
                }
            }
            // If the current state depends on itself, then it should update itself when it is invalidated
            let dependants = Arc::get_mut(&mut current.dependants).unwrap();
            let current_dependant = Dependant {
                type_id: current.this_type_id,
                enter_shadow_dom: current.enter_shadow_dom,
            };
            match current.pass_direction {
                PassDirection::ChildToParent => {
                    if !dependants.parent.contains(&current_dependant) {
                        dependants.parent.push(current_dependant);
                    }
                }
                PassDirection::ParentToChild => {
                    if !dependants.child.contains(&current_dependant) {
                        dependants.child.push(current_dependant);
                    }
                }
                _ => {}
            }
        }
        let workload = construct_workload(&mut tracked_states);
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

        passes_updated.insert(
            root_id,
            tracked_states.iter().map(|x| x.this_type_id).collect(),
        );
        nodes_updated.insert(root_id, NodeMaskBuilder::ALL.build());

        RealDom {
            world,
            nodes_listening: FxHashMap::default(),
            dirty_nodes: NodesDirty {
                passes_updated,
                nodes_updated,
                passes: tracked_states,
                nodes_created: [root_id].into_iter().collect(),
            },
            node_watchers: Default::default(),
            attribute_watchers: Default::default(),
            workload,
            root_id,
            custom_elements: Default::default(),
            phantom: std::marker::PhantomData,
        }
    }

    /// Get a reference to the tree.
    pub fn tree_ref(&self) -> TreeRefView {
        self.world.borrow().unwrap()
    }

    /// Get a mutable reference to the tree.
    pub fn tree_mut(&self) -> TreeMutView {
        self.world.borrow().unwrap()
    }

    /// Create a new node of the given type in the dom and return a mutable reference to it.
    pub fn create_node(&mut self, node: impl Into<NodeType<V>>) -> NodeMut<'_, V> {
        let node = node.into();
        let is_element = matches!(node, NodeType::Element(_));

        let id = self.world.add_entity(node);
        self.tree_mut().create_node(id);

        self.dirty_nodes
            .passes_updated
            .entry(id)
            .or_default()
            .extend(self.dirty_nodes.passes.iter().map(|x| x.this_type_id));
        self.dirty_nodes
            .mark_dirty(id, NodeMaskBuilder::ALL.build());
        self.dirty_nodes.nodes_created.insert(id);

        // Create a custom element if needed
        if is_element {
            let custom_elements = self.custom_elements.clone();
            custom_elements
                .read()
                .unwrap()
                .add_shadow_dom(NodeMut::new(id, self));
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

    /// Check if a node exists in the dom.
    pub fn contains(&self, id: NodeId) -> bool {
        self.tree_ref().contains(id)
    }

    /// Get a reference to a node.
    pub fn get(&self, id: NodeId) -> Option<NodeRef<'_, V>> {
        self.contains(id).then_some(NodeRef { id, dom: self })
    }

    /// Get a mutable reference to a node.
    pub fn get_mut(&mut self, id: NodeId) -> Option<NodeMut<'_, V>> {
        let contains = self.contains(id);
        contains.then(|| NodeMut::new(id, self))
    }

    /// Borrow a component from the world without updating the dirty nodes.
    fn borrow_raw<'a, B: IntoBorrow>(&'a self) -> Result<B, GetStorage>
    where
        B::Borrow: shipyard::Borrow<'a, View = B>,
    {
        self.world.borrow()
    }

    /// Borrow a component from the world without updating the dirty nodes.
    fn borrow_node_type_mut(&self) -> Result<ViewMut<NodeType<V>>, GetStorage> {
        self.world.borrow()
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state(
        &mut self,
        ctx: SendAnyMap,
    ) -> (FxDashSet<NodeId>, FxHashMap<NodeId, NodeMask>) {
        let nodes_created = std::mem::take(&mut self.dirty_nodes.nodes_created);

        // call node watchers
        {
            let watchers = self.node_watchers.clone();

            // ignore watchers if they are already being modified
            if let Ok(mut watchers) = watchers.try_write() {
                for id in &nodes_created {
                    for watcher in &mut *watchers {
                        watcher.on_node_added(NodeMut::new(*id, self));
                    }
                }
            };
        }

        let passes = std::mem::take(&mut self.dirty_nodes.passes_updated);
        let nodes_updated = std::mem::take(&mut self.dirty_nodes.nodes_updated);

        for (node_id, mask) in &nodes_updated {
            if self.contains(*node_id) {
                // call attribute watchers but ignore watchers if they are already being modified
                let watchers = self.attribute_watchers.clone();
                if let Ok(mut watchers) = watchers.try_write() {
                    for watcher in &mut *watchers {
                        watcher.on_attributes_changed(
                            self.get_mut(*node_id).unwrap(),
                            mask.attributes(),
                        );
                    }
                };

                // call custom element watchers
                let node = self.get_mut(*node_id).unwrap();
                let custom_element_manager =
                    node.get::<CustomElementManager<V>>().map(|x| x.clone());
                if let Some(custom_element_manager) = custom_element_manager {
                    custom_element_manager.on_attributes_changed(node, mask.attributes());
                }
            }
        }

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

    /// Traverses the dom in a depth first manner, calling the provided function on each node.
    /// If `enter_shadow_dom` is true, then the traversal will enter shadow doms in the tree.
    pub fn traverse_depth_first_advanced(
        &self,
        enter_shadow_dom: bool,
        mut f: impl FnMut(NodeRef<V>),
    ) {
        let mut stack = vec![self.root_id()];
        let tree = self.tree_ref();
        while let Some(id) = stack.pop() {
            if let Some(node) = self.get(id) {
                f(node);
                let children = tree.children_ids_advanced(id, enter_shadow_dom);
                stack.extend(children.iter().copied().rev());
            }
        }
    }

    /// Traverses the dom in a depth first manner, calling the provided function on each node.
    pub fn traverse_depth_first(&self, f: impl FnMut(NodeRef<V>)) {
        self.traverse_depth_first_advanced(true, f)
    }

    /// Traverses the dom in a breadth first manner, calling the provided function on each node.
    /// If `enter_shadow_dom` is true, then the traversal will enter shadow doms in the tree.
    pub fn traverse_breadth_first_advanced(
        &self,
        enter_shadow_doms: bool,
        mut f: impl FnMut(NodeRef<V>),
    ) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root_id());
        let tree = self.tree_ref();
        while let Some(id) = queue.pop_front() {
            if let Some(node) = self.get(id) {
                f(node);
                let children = tree.children_ids_advanced(id, enter_shadow_doms);
                for id in children {
                    queue.push_back(id);
                }
            }
        }
    }

    /// Traverses the dom in a breadth first manner, calling the provided function on each node.
    pub fn traverse_breadth_first(&self, f: impl FnMut(NodeRef<V>)) {
        self.traverse_breadth_first_advanced(true, f);
    }

    /// Traverses the dom in a depth first manner mutably, calling the provided function on each node.
    /// If `enter_shadow_dom` is true, then the traversal will enter shadow doms in the tree.
    pub fn traverse_depth_first_mut_advanced(
        &mut self,
        enter_shadow_doms: bool,
        mut f: impl FnMut(NodeMut<V>),
    ) {
        let mut stack = vec![self.root_id()];
        while let Some(id) = stack.pop() {
            let tree = self.tree_ref();
            let mut children = tree.children_ids_advanced(id, enter_shadow_doms);
            drop(tree);
            children.reverse();
            if let Some(node) = self.get_mut(id) {
                f(node);
                stack.extend(children.iter());
            }
        }
    }

    /// Traverses the dom in a depth first manner mutably, calling the provided function on each node.
    pub fn traverse_depth_first_mut(&mut self, f: impl FnMut(NodeMut<V>)) {
        self.traverse_depth_first_mut_advanced(true, f)
    }

    /// Traverses the dom in a breadth first manner mutably, calling the provided function on each node.
    /// If `enter_shadow_dom` is true, then the traversal will enter shadow doms in the tree.
    pub fn traverse_breadth_first_mut_advanced(
        &mut self,
        enter_shadow_doms: bool,
        mut f: impl FnMut(NodeMut<V>),
    ) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root_id());
        while let Some(id) = queue.pop_front() {
            let tree = self.tree_ref();
            let children = tree.children_ids_advanced(id, enter_shadow_doms);
            drop(tree);
            if let Some(node) = self.get_mut(id) {
                f(node);
                for id in children {
                    queue.push_back(id);
                }
            }
        }
    }

    /// Traverses the dom in a breadth first manner mutably, calling the provided function on each node.
    pub fn traverse_breadth_first_mut(&mut self, f: impl FnMut(NodeMut<V>)) {
        self.traverse_breadth_first_mut_advanced(true, f);
    }

    /// Adds a [`NodeWatcher`] to the dom. Node watchers are called whenever a node is created or removed.
    pub fn add_node_watcher(&mut self, watcher: impl NodeWatcher<V> + 'static + Send + Sync) {
        self.node_watchers.write().unwrap().push(Box::new(watcher));
    }

    /// Adds an [`AttributeWatcher`] to the dom. Attribute watchers are called whenever an attribute is changed.
    pub fn add_attribute_watcher(
        &mut self,
        watcher: impl AttributeWatcher<V> + 'static + Send + Sync,
    ) {
        self.attribute_watchers
            .write()
            .unwrap()
            .push(Box::new(watcher));
    }

    /// Returns a reference to the underlying world. Any changes made to the world will not update the reactive system.
    pub fn raw_world(&self) -> &World {
        &self.world
    }

    /// Returns a mutable reference to the underlying world. Any changes made to the world will not update the reactive system.
    pub fn raw_world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Registers a new custom element.
    pub fn register_custom_element<E: CustomElement<V>>(&mut self) {
        self.register_custom_element_with_factory::<E, E>()
    }

    /// Registers a new custom element with a custom factory.
    pub fn register_custom_element_with_factory<F, U>(&mut self)
    where
        F: CustomElementFactory<U, V>,
        U: CustomElementUpdater<V>,
    {
        self.custom_elements.write().unwrap().register::<F, U>()
    }
}

/// A reference to a tracked component in a node.
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

/// A mutable reference to a tracked component in a node.
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

/// A immutable view of a node
pub trait NodeImmutable<V: FromAnyValue + Send + Sync = ()>: Sized {
    /// Get the real dom this node was created in
    fn real_dom(&self) -> &RealDom<V>;

    /// Get the id of the current node
    fn id(&self) -> NodeId;

    /// Get the type of the current node
    #[inline]
    fn node_type(&self) -> ViewEntry<NodeType<V>> {
        self.get().unwrap()
    }

    /// Get a component from the current node
    #[inline]
    fn get<'a, T: Component + Sync + Send>(&'a self) -> Option<ViewEntry<'a, T>> {
        // self.real_dom().tree.get(self.id())
        let view: View<'a, T> = self.real_dom().borrow_raw().ok()?;
        view.contains(self.id())
            .then(|| ViewEntry::new(view, self.id()))
    }

    /// Get the ids of the children of the current node, if enter_shadow_dom is true and the current node is a shadow slot, the ids of the nodes under the node the shadow slot is attached to will be returned
    #[inline]
    fn children_ids_advanced(&self, id: NodeId, enter_shadow_dom: bool) -> Vec<NodeId> {
        self.real_dom()
            .tree_ref()
            .children_ids_advanced(id, enter_shadow_dom)
    }

    /// Get the ids of the children of the current node
    #[inline]
    fn child_ids(&self) -> Vec<NodeId> {
        self.real_dom().tree_ref().children_ids(self.id())
    }

    /// Get the children of the current node
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

    /// Get the id of the parent of the current node, if enter_shadow_dom is true and the current node is a shadow root, the node the shadow root is attached to will be returned
    #[inline]
    fn parent_id_advanced(&self, id: NodeId, enter_shadow_dom: bool) -> Option<NodeId> {
        self.real_dom()
            .tree_ref()
            .parent_id_advanced(id, enter_shadow_dom)
    }

    /// Get the id of the parent of the current node
    #[inline]
    fn parent_id(&self) -> Option<NodeId> {
        self.real_dom().tree_ref().parent_id(self.id())
    }

    /// Get the parent of the current node
    #[inline]
    fn parent(&self) -> Option<NodeRef<V>> {
        self.parent_id().map(|id| NodeRef {
            id,
            dom: self.real_dom(),
        })
    }

    /// Get the node after the current node
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

    /// Get the node before the current node
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

    /// Get the height of the current node in the tree (the number of nodes between the current node and the root)
    #[inline]
    fn height(&self) -> u16 {
        self.real_dom().tree_ref().height(self.id()).unwrap()
    }
}

/// An immutable reference to a node in a RealDom
pub struct NodeRef<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    dom: &'a RealDom<V>,
}

impl<'a, V: FromAnyValue + Send + Sync> Clone for NodeRef<'a, V> {
    fn clone(&self) -> Self {
        *self
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

/// A mutable refrence to a node in the RealDom that tracks what States need to be updated
pub struct NodeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    dom: &'a mut RealDom<V>,
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMut<'a, V> {
    /// Create a new mutable refrence to a node in a RealDom
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
    /// Reborrow the node mutably
    pub fn reborrow(&mut self) -> NodeMut<'_, V> {
        NodeMut {
            id: self.id,
            dom: self.dom,
        }
    }

    /// Get the real dom this node was created in mutably
    #[inline(always)]
    pub fn real_dom_mut(&mut self) -> &mut RealDom<V> {
        self.dom
    }

    /// Get the parent of this node mutably
    #[inline]
    pub fn parent_mut(&mut self) -> Option<NodeMut<V>> {
        self.parent_id().map(|id| NodeMut { id, dom: self.dom })
    }

    /// Get a component from the current node mutably
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
        view_mut
            .contains(self.id)
            .then_some(ViewEntryMut::new(view_mut, self.id))
    }

    /// Insert a custom component into this node
    ///
    /// Note: Components that implement State and are added when the RealDom is created will automatically be created
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

    /// Get the next node
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

    /// Get the previous node
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

    /// Add the given node to the end of this nodes children
    #[inline]
    pub fn add_child(&mut self, child: NodeId) {
        self.dom.dirty_nodes.mark_child_changed(self.id);
        self.dom.dirty_nodes.mark_parent_added_or_removed(child);
        self.dom.tree_mut().add_child(self.id, child);
        NodeMut::new(child, self.dom).mark_moved();
    }

    /// Insert this node after the given node
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

    /// Insert this node before the given node
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

    /// Remove this node from the RealDom
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
        self.dom.tree_mut().remove(id);
        self.real_dom_mut().raw_world_mut().delete_entity(id);
    }

    /// Replace this node with a different node
    #[inline]
    pub fn replace(mut self, new: NodeId) {
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
        self.remove();
    }

    /// Add an event listener
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

    /// Remove an event listener
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

    /// mark that this node was removed for the incremental system
    fn mark_removed(&mut self) {
        let watchers = self.dom.node_watchers.clone();
        for watcher in &mut *watchers.write().unwrap() {
            watcher.on_node_removed(NodeMut::new(self.id(), self.dom));
        }
    }

    /// mark that this node was moved for the incremental system
    fn mark_moved(&mut self) {
        let watchers = self.dom.node_watchers.clone();
        // ignore watchers if the we are inside of a watcher
        if let Ok(mut watchers) = watchers.try_write() {
            for watcher in &mut *watchers {
                watcher.on_node_moved(NodeMut::new(self.id(), self.dom));
            }
        };
    }

    /// Get a mutable reference to the type of the current node
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

    /// Set the type of the current node
    pub fn set_type(&mut self, new: NodeType<V>) {
        {
            let mut view: ViewMut<NodeType<V>> = self.dom.borrow_node_type_mut().unwrap();
            *(&mut view).get(self.id).unwrap() = new;
        }
        self.dom
            .dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::ALL.build())
    }

    /// Clone a node and it's children and returns the id of the new node.
    /// This is more effecient than creating the node from scratch because it can pre-allocate the memory required.
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

/// A mutable refrence to the type of a node in the RealDom
pub enum NodeTypeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    /// An element node
    Element(ElementNodeMut<'a, V>),
    /// A text node
    Text(TextNodeMut<'a, V>),
    /// A placeholder node
    Placeholder,
}

/// A mutable refrence to a text node in the RealDom
pub struct TextNodeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    text: ViewEntryMut<'a, NodeType<V>>,
    dirty_nodes: &'a mut NodesDirty<V>,
}

impl<V: FromAnyValue + Send + Sync> TextNodeMut<'_, V> {
    /// Get the underlying test of the node
    pub fn text(&self) -> &str {
        match &*self.text {
            NodeType::Text(text) => &text.text,
            _ => unreachable!(),
        }
    }

    /// Get the underlying text mutably
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

/// A mutable refrence to a text Element node in the RealDom
pub struct ElementNodeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    element: ViewEntryMut<'a, NodeType<V>>,
    dirty_nodes: &'a mut NodesDirty<V>,
}

impl std::fmt::Debug for ElementNodeMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElementNodeMut")
            .field("id", &self.id)
            .field("element", &*self.element)
            .finish()
    }
}

impl<V: FromAnyValue + Send + Sync> ElementNodeMut<'_, V> {
    /// Get the current element
    fn element(&self) -> &ElementNode<V> {
        match &*self.element {
            NodeType::Element(element) => element,
            _ => unreachable!(),
        }
    }

    /// Get the current element mutably (does not mark anything as dirty)
    fn element_mut(&mut self) -> &mut ElementNode<V> {
        match &mut *self.element {
            NodeType::Element(element) => element,
            _ => unreachable!(),
        }
    }

    /// Get the tag of the element
    pub fn tag(&self) -> &str {
        &self.element().tag
    }

    /// Get a mutable reference to the tag of the element
    pub fn tag_mut(&mut self) -> &mut String {
        self.dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::new().with_tag().build());
        &mut self.element_mut().tag
    }

    /// Get a reference to the namespace the element is in
    pub fn namespace(&self) -> Option<&str> {
        self.element().namespace.as_deref()
    }

    /// Get a mutable reference to the namespace the element is in
    pub fn namespace_mut(&mut self) -> &mut Option<String> {
        self.dirty_nodes
            .mark_dirty(self.id, NodeMaskBuilder::new().with_namespace().build());
        &mut self.element_mut().namespace
    }

    /// Get a reference to all of the attributes currently set on the element
    pub fn attributes(&self) -> &FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>> {
        &self.element().attributes
    }

    /// Set an attribute in the element
    pub fn set_attribute(
        &mut self,
        name: impl Into<OwnedAttributeDiscription>,
        value: impl Into<OwnedAttributeValue<V>>,
    ) -> Option<OwnedAttributeValue<V>> {
        let name = name.into();
        let value = value.into();
        self.dirty_nodes.mark_dirty(
            self.id,
            NodeMaskBuilder::new()
                .with_attrs(AttributeMaskBuilder::Some(&[&name.name]))
                .build(),
        );
        self.element_mut().attributes.insert(name, value)
    }

    /// Remove an attribute from the element
    pub fn remove_attribute(
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

    /// Get an attribute of the element
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

    /// Get an attribute of the element
    pub fn get_attribute(
        &self,
        name: &OwnedAttributeDiscription,
    ) -> Option<&OwnedAttributeValue<V>> {
        self.element().attributes.get(name)
    }

    /// Get the set of all events the element is listening to
    pub fn listeners(&self) -> &FxHashSet<String> {
        &self.element().listeners
    }
}

// Create a workload from all of the passes. This orders the passes so that each pass will only run at most once.
fn construct_workload<V: FromAnyValue + Send + Sync>(
    passes: &mut [TypeErasedState<V>],
) -> Workload {
    let mut workload = Workload::new("Main Workload");
    // Assign a unique index to keep track of each pass
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
        let all_dependancies: Vec<_> = pass.combined_dependancy_type_ids().collect();
        for ty_id in all_dependancies {
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
    // Add all of the passes
    for (_, _, mut workload_system) in unresloved_workloads {
        workload = workload.with_system(workload_system.take().unwrap());
    }
    workload
}

use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHashSet};
use shipyard::{Borrow, BorrowInfo, Component, Unique, UniqueView, View, WorkloadSystem};
use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

use crate::node::{FromAnyValue, NodeType};
use crate::node_ref::{NodeMaskBuilder, NodeView};
use crate::real_dom::{DirtyNodesResult, SendAnyMapWrapper};
use crate::tree::{TreeRef, TreeRefView};
use crate::SendAnyMap;
use crate::{NodeId, NodeMask};

#[derive(Default)]
struct DirtyNodes {
    nodes_dirty: FxHashSet<NodeId>,
}

impl DirtyNodes {
    pub fn add_node(&mut self, node_id: NodeId) {
        self.nodes_dirty.insert(node_id);
    }

    pub fn is_empty(&self) -> bool {
        self.nodes_dirty.is_empty()
    }

    pub fn pop(&mut self) -> Option<NodeId> {
        self.nodes_dirty.iter().next().copied().map(|id| {
            self.nodes_dirty.remove(&id);
            id
        })
    }
}

/// Tracks the dirty nodes sorted by height for each pass. We resolve passes based on the height of the node in order to avoid resolving any node twice in a pass.
#[derive(Clone, Unique)]
pub struct DirtyNodeStates {
    dirty: Arc<FxHashMap<TypeId, RwLock<BTreeMap<u16, DirtyNodes>>>>,
}

impl DirtyNodeStates {
    pub fn with_passes(passes: impl Iterator<Item = TypeId>) -> Self {
        Self {
            dirty: Arc::new(
                passes
                    .map(|pass| (pass, RwLock::new(BTreeMap::new())))
                    .collect(),
            ),
        }
    }

    pub fn insert(&self, pass_id: TypeId, node_id: NodeId, height: u16) {
        if let Some(btree) = self.dirty.get(&pass_id) {
            let mut write = btree.write();
            if let Some(entry) = write.get_mut(&height) {
                entry.add_node(node_id);
            } else {
                let mut entry = DirtyNodes::default();
                entry.add_node(node_id);
                write.insert(height, entry);
            }
        }
    }

    fn pop_front(&self, pass_id: TypeId) -> Option<(u16, NodeId)> {
        let mut values = self.dirty.get(&pass_id)?.write();
        let mut value = values.first_entry()?;
        let height = *value.key();
        let ids = value.get_mut();
        let id = ids.pop()?;
        if ids.is_empty() {
            value.remove_entry();
        }

        Some((height, id))
    }

    fn pop_back(&self, pass_id: TypeId) -> Option<(u16, NodeId)> {
        let mut values = self.dirty.get(&pass_id)?.write();
        let mut value = values.last_entry()?;
        let height = *value.key();
        let ids = value.get_mut();
        let id = ids.pop()?;
        if ids.is_empty() {
            value.remove_entry();
        }

        Some((height, id))
    }
}

/// A state that is automatically inserted in a node with dependencies.
pub trait State<V: FromAnyValue + Send + Sync = ()>: Any + Send + Sync {
    /// This is a tuple of (T: State, ..) of states read from the parent required to update this state
    type ParentDependencies: Dependancy;
    /// This is a tuple of (T: State, ..) of states read from the children required to update this state
    type ChildDependencies: Dependancy;
    /// This is a tuple of (T: State, ..) of states read from the node required to update this state
    type NodeDependencies: Dependancy;
    /// This is a mask of what aspects of the node are required to update this state
    const NODE_MASK: NodeMaskBuilder<'static>;

    /// Does the state traverse into the shadow dom or pass over it. This should be true for layout and false for styles
    const TRAVERSE_SHADOW_DOM: bool = false;

    /// Update this state in a node, returns if the state was updated
    fn update<'a>(
        &mut self,
        node_view: NodeView<V>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> bool;

    /// Create a new instance of this state
    fn create<'a>(
        node_view: NodeView<V>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> Self;

    /// Create a workload system for this state
    fn workload_system(
        type_id: TypeId,
        dependants: Arc<Dependants>,
        pass_direction: PassDirection,
    ) -> WorkloadSystem;

    /// Converts to a type erased version of the trait
    fn to_type_erased() -> TypeErasedState<V>
    where
        Self: Sized,
    {
        let node_mask = Self::NODE_MASK.build();
        TypeErasedState {
            this_type_id: TypeId::of::<Self>(),
            parent_dependancies_ids: Self::ParentDependencies::type_ids()
                .iter()
                .copied()
                .collect(),
            child_dependancies_ids: Self::ChildDependencies::type_ids()
                .iter()
                .copied()
                .collect(),
            node_dependancies_ids: Self::NodeDependencies::type_ids().iter().copied().collect(),
            dependants: Default::default(),
            mask: node_mask,
            pass_direction: pass_direction::<V, Self>(),
            enter_shadow_dom: Self::TRAVERSE_SHADOW_DOM,
            workload: Self::workload_system,
            phantom: PhantomData,
        }
    }
}

fn pass_direction<V: FromAnyValue + Send + Sync, S: State<V>>() -> PassDirection {
    if S::ChildDependencies::type_ids()
        .iter()
        .any(|type_id| *type_id == TypeId::of::<S>())
    {
        PassDirection::ChildToParent
    } else if S::ParentDependencies::type_ids()
        .iter()
        .any(|type_id| *type_id == TypeId::of::<S>())
    {
        PassDirection::ParentToChild
    } else {
        PassDirection::AnyOrder
    }
}

#[doc(hidden)]
#[derive(Borrow, BorrowInfo)]
pub struct RunPassView<'a, V: FromAnyValue + Send + Sync = ()> {
    pub tree: TreeRefView<'a>,
    pub node_type: View<'a, NodeType<V>>,
    dirty_nodes_result: UniqueView<'a, DirtyNodesResult>,
    node_states: UniqueView<'a, DirtyNodeStates>,
    any_map: UniqueView<'a, SendAnyMapWrapper>,
}

// This is used by the macro
/// Updates the given pass, marking any nodes that were changed
#[doc(hidden)]
pub fn run_pass<V: FromAnyValue + Send + Sync>(
    type_id: TypeId,
    dependants: Arc<Dependants>,
    pass_direction: PassDirection,
    view: RunPassView<V>,
    mut update_node: impl FnMut(NodeId, &SendAnyMap) -> bool,
) {
    let RunPassView {
        tree,
        dirty_nodes_result: nodes_updated,
        node_states: dirty,
        any_map: ctx,
        ..
    } = view;
    let ctx = ctx.as_ref();
    match pass_direction {
        PassDirection::ParentToChild => {
            while let Some((height, id)) = dirty.pop_front(type_id) {
                if (update_node)(id, ctx) {
                    nodes_updated.insert(id);
                    dependants.mark_dirty(&dirty, id, &tree, height);
                }
            }
        }
        PassDirection::ChildToParent => {
            while let Some((height, id)) = dirty.pop_back(type_id) {
                if (update_node)(id, ctx) {
                    nodes_updated.insert(id);
                    dependants.mark_dirty(&dirty, id, &tree, height);
                }
            }
        }
        PassDirection::AnyOrder => {
            while let Some((height, id)) = dirty.pop_back(type_id) {
                if (update_node)(id, ctx) {
                    nodes_updated.insert(id);
                    dependants.mark_dirty(&dirty, id, &tree, height);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Dependant {
    pub(crate) type_id: TypeId,
    pub(crate) enter_shadow_dom: bool,
}

/// The states that depend on this state
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Dependants {
    /// The states in the parent direction that should be invalidated when this state is invalidated
    pub(crate) parent: Vec<Dependant>,
    /// The states in the child direction that should be invalidated when this state is invalidated
    pub(crate) child: Vec<Dependant>,
    /// The states in the node direction that should be invalidated when this state is invalidated
    pub(crate) node: Vec<TypeId>,
}

impl Dependants {
    fn mark_dirty(&self, dirty: &DirtyNodeStates, id: NodeId, tree: &impl TreeRef, height: u16) {
        for &Dependant {
            type_id,
            enter_shadow_dom,
        } in &self.child
        {
            for id in tree.children_ids_advanced(id, enter_shadow_dom) {
                dirty.insert(type_id, id, height + 1);
            }
        }

        for &Dependant {
            type_id,
            enter_shadow_dom,
        } in &self.parent
        {
            if let Some(id) = tree.parent_id_advanced(id, enter_shadow_dom) {
                dirty.insert(type_id, id, height - 1);
            }
        }

        for dependant in &self.node {
            dirty.insert(*dependant, id, height);
        }
    }
}

/// A type erased version of [`State`] that can be added to the [`crate::prelude::RealDom`] with [`crate::prelude::RealDom::new`]
pub struct TypeErasedState<V: FromAnyValue + Send = ()> {
    pub(crate) this_type_id: TypeId,
    pub(crate) parent_dependancies_ids: FxHashSet<TypeId>,
    pub(crate) child_dependancies_ids: FxHashSet<TypeId>,
    pub(crate) node_dependancies_ids: FxHashSet<TypeId>,
    pub(crate) dependants: Arc<Dependants>,
    pub(crate) mask: NodeMask,
    pub(crate) workload: fn(TypeId, Arc<Dependants>, PassDirection) -> WorkloadSystem,
    pub(crate) pass_direction: PassDirection,
    pub(crate) enter_shadow_dom: bool,
    phantom: PhantomData<V>,
}

impl<V: FromAnyValue + Send> TypeErasedState<V> {
    pub(crate) fn create_workload(&self) -> WorkloadSystem {
        (self.workload)(
            self.this_type_id,
            self.dependants.clone(),
            self.pass_direction,
        )
    }

    pub(crate) fn combined_dependancy_type_ids(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.parent_dependancies_ids
            .iter()
            .chain(self.child_dependancies_ids.iter())
            .chain(self.node_dependancies_ids.iter())
            .copied()
    }
}

/// The direction that a pass should be run in
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum PassDirection {
    /// The pass should be run from the root to the leaves
    ParentToChild,
    /// The pass should be run from the leaves to the root
    ChildToParent,
    /// The pass can be run in any order
    AnyOrder,
}

/// A trait that is implemented for all the dependancies of a [`State`]
pub trait Dependancy {
    /// A tuple with all the elements of the dependancy as [`DependancyView`]
    type ElementBorrowed<'a>;

    /// Returns a list of all the [`TypeId`]s of the elements in the dependancy
    fn type_ids() -> Box<[TypeId]> {
        Box::new([])
    }
}

macro_rules! impl_dependancy {
    ($($t:ident),*) => {
        impl< $($t: Send + Sync + Component),* > Dependancy for ($($t,)*) {
            type ElementBorrowed<'a> = ($(DependancyView<'a, $t>,)*);

            fn type_ids() -> Box<[TypeId]> {
                Box::new([$(TypeId::of::<$t>()),*])
            }
        }
    };
}

// TODO: track what components are actually read to update subscriptions
// making this a wrapper makes it possible to implement that optimization without a breaking change
/// A immutable view of a [`State`]
pub struct DependancyView<'a, T> {
    inner: &'a T,
}

impl<'a, T> DependancyView<'a, T> {
    // This should only be used in the macro. This is not a public API or stable
    #[doc(hidden)]
    pub fn new(inner: &'a T) -> Self {
        Self { inner }
    }
}

impl<'a, T> Deref for DependancyView<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl_dependancy!();
impl_dependancy!(A);
impl_dependancy!(A, B);
impl_dependancy!(A, B, C);
impl_dependancy!(A, B, C, D);
impl_dependancy!(A, B, C, D, E);
impl_dependancy!(A, B, C, D, E, F);
impl_dependancy!(A, B, C, D, E, F, G);
impl_dependancy!(A, B, C, D, E, F, G, H);
impl_dependancy!(A, B, C, D, E, F, G, H, I);
impl_dependancy!(A, B, C, D, E, F, G, H, I, J);

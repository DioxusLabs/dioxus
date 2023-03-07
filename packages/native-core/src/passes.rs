use anymap::AnyMap;
use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHashSet};
use shipyard::{
    BorrowInfo, Component, IntoBorrow, IntoWorkloadSystem, Unique, UniqueView, View,
    WorkloadSystem, World,
};
use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::node::FromAnyValue;
use crate::node_ref::{NodeMaskBuilder, NodeView};
use crate::real_dom::{DirtyNodesResult, SendAnyMapWrapper};
use crate::tree::{TreeMut, TreeRef, TreeRefView};
use crate::{FxDashSet, SendAnyMap};
use crate::{NodeId, NodeMask};

#[derive(Default)]
struct DirtyNodes {
    passes_dirty: Vec<u64>,
}

impl DirtyNodes {
    pub fn add_node(&mut self, node_id: NodeId) {
        let node_id = node_id.uindex();
        let index = node_id / 64;
        let bit = node_id % 64;
        let encoded = 1 << bit;
        if let Some(passes) = self.passes_dirty.get_mut(index) {
            *passes |= encoded;
        } else {
            self.passes_dirty.resize(index + 1, 0);
            self.passes_dirty[index] |= encoded;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.passes_dirty.iter().all(|dirty| *dirty == 0)
    }

    pub fn pop(&mut self) -> Option<usize> {
        let index = self.passes_dirty.iter().position(|dirty| *dirty != 0)?;
        let passes = self.passes_dirty[index];
        let node_id = passes.trailing_zeros();
        let encoded = 1 << node_id;
        self.passes_dirty[index] &= !encoded;
        Some((index * 64) + node_id as usize)
    }
}

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

    fn pop_front(&self, pass_id: TypeId) -> Option<(u16, usize)> {
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

    fn pop_back(&self, pass_id: TypeId) -> Option<(u16, usize)> {
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

pub trait State<V: FromAnyValue + Send + Sync = ()>: Any + Send + Sync {
    /// This is a tuple of (T: Pass, ..) of states read from the parent required to run this pass
    type ParentDependencies: Dependancy;
    /// This is a tuple of (T: Pass, ..) of states read from the children required to run this pass
    type ChildDependencies: Dependancy;
    /// This is a tuple of (T: Pass, ..) of states read from the node required to run this pass
    type NodeDependencies: Dependancy;
    /// A tuple of all the dependencies combined
    type CombinedDependencies: Dependancy;
    /// This is a mask of what aspects of the node are required to run this pass
    const NODE_MASK: NodeMaskBuilder<'static>;

    /// Update this state in a node, returns if the state was updated
    fn update<'a>(
        &mut self,
        node_view: NodeView<V>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
        context: &SendAnyMap,
    ) -> bool;

    /// Create a new instance of this state
    fn create<'a>(
        node_view: NodeView<V>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
        context: &SendAnyMap,
    ) -> Self;

    /// Create a workload system for this state
    fn workload_system(dependants: FxHashSet<TypeId>) -> WorkloadSystem;
}

pub struct RunPassView<'a> {
    type_id: TypeId,
    dependants: FxHashSet<TypeId>,
    pass_direction: PassDirection,
    tree: TreeRefView<'a>,
    nodes_updated: UniqueView<'a, DirtyNodesResult>,
    dirty: UniqueView<'a, DirtyNodeStates>,
    ctx: UniqueView<'a, SendAnyMapWrapper>,
}

impl<'a> RunPassView<'a> {
    pub fn borrow(
        type_id: TypeId,
        dependants: FxHashSet<TypeId>,
        pass_direction: PassDirection,
        world: &'a World,
    ) -> Self {
        Self {
            type_id,
            dependants,
            pass_direction,
            tree: world.borrow().unwrap(),
            nodes_updated: world.borrow().unwrap(),
            dirty: world.borrow().unwrap(),
            ctx: world.borrow().unwrap(),
        }
    }
}

pub fn run_pass(view: RunPassView, mut update_node: impl FnMut(NodeId, &SendAnyMap) -> bool) {
    let RunPassView {
        type_id,
        dependants,
        pass_direction,
        tree,
        nodes_updated,
        dirty,
        ctx,
    } = view;
    let ctx = ctx.as_ref();
    match pass_direction {
        PassDirection::ParentToChild => {
            while let Some((height, id)) = dirty.pop_front(type_id) {
                let id = tree.id_at(id).unwrap();
                if (update_node)(id, ctx) {
                    nodes_updated.insert(id);
                    for id in tree.children_ids(id).unwrap() {
                        for dependant in &dependants {
                            dirty.insert(*dependant, id, height + 1);
                        }
                    }
                }
            }
        }
        PassDirection::ChildToParent => {
            while let Some((height, id)) = dirty.pop_back(type_id) {
                let id = tree.id_at(id).unwrap();
                if (update_node)(id, ctx) {
                    nodes_updated.insert(id);
                    if let Some(id) = tree.parent_id(id) {
                        for dependant in &dependants {
                            dirty.insert(*dependant, id, height - 1);
                        }
                    }
                }
            }
        }
        PassDirection::AnyOrder => {
            while let Some((height, id)) = dirty.pop_back(type_id) {
                let id = tree.id_at(id).unwrap();
                if (update_node)(id, ctx) {
                    nodes_updated.insert(id);
                    for dependant in &dependants {
                        dirty.insert(*dependant, id, height);
                    }
                }
            }
        }
    }
}

pub trait AnyState<V: FromAnyValue + Send + Sync = ()>: State<V> {
    fn to_type_erased() -> TypeErasedPass<V>
    where
        Self: Sized,
    {
        let node_mask = Self::NODE_MASK.build();
        TypeErasedPass {
            this_type_id: TypeId::of::<Self>(),
            combined_dependancy_type_ids: Self::all_dependanices().iter().copied().collect(),
            parent_dependant: !Self::parent_type_ids().is_empty(),
            child_dependant: !Self::child_type_ids().is_empty(),
            dependants: FxHashSet::default(),
            mask: node_mask,
            pass_direction: Self::pass_direction(),
            workload: Self::workload_system,
            phantom: PhantomData,
        }
    }

    fn parent_type_ids() -> Box<[TypeId]> {
        Self::ParentDependencies::type_ids()
    }

    fn child_type_ids() -> Box<[TypeId]> {
        Self::ChildDependencies::type_ids()
    }

    fn node_type_ids() -> Box<[TypeId]> {
        Self::NodeDependencies::type_ids()
    }

    fn all_dependanices() -> Box<[TypeId]> {
        let mut dependencies = Self::parent_type_ids().to_vec();
        dependencies.extend(Self::child_type_ids().iter());
        dependencies.extend(Self::node_type_ids().iter());
        dependencies.into_boxed_slice()
    }

    fn pass_direction() -> PassDirection {
        if Self::child_type_ids()
            .iter()
            .any(|type_id| *type_id == TypeId::of::<Self>())
        {
            PassDirection::ChildToParent
        } else if Self::parent_type_ids()
            .iter()
            .any(|type_id| *type_id == TypeId::of::<Self>())
        {
            PassDirection::ParentToChild
        } else {
            PassDirection::AnyOrder
        }
    }
}

pub struct TypeErasedPass<V: FromAnyValue + Send = ()> {
    pub(crate) this_type_id: TypeId,
    pub(crate) parent_dependant: bool,
    pub(crate) child_dependant: bool,
    pub(crate) combined_dependancy_type_ids: FxHashSet<TypeId>,
    pub(crate) dependants: FxHashSet<TypeId>,
    pub(crate) mask: NodeMask,
    pub(crate) workload: fn(FxHashSet<TypeId>) -> WorkloadSystem,
    pub(crate) pass_direction: PassDirection,
    phantom: PhantomData<V>,
}

#[derive(Debug)]
pub enum PassDirection {
    ParentToChild,
    ChildToParent,
    AnyOrder,
}

pub trait AnyMapLike<'a> {
    fn get<T: Any + Sync + Send>(self) -> Option<&'a T>;
}

impl<'a> AnyMapLike<'a> for &'a AnyMap {
    fn get<T: Any + Sync + Send>(self) -> Option<&'a T> {
        self.get()
    }
}

impl<'a> AnyMapLike<'a> for &'a SendAnyMap {
    fn get<T: Any + Sync + Send>(self) -> Option<&'a T> {
        todo!()
    }
}

pub trait Dependancy {
    type ElementBorrowed<'a>: IntoBorrow + BorrowInfo;

    fn type_ids() -> Box<[TypeId]> {
        Box::new([])
    }
}

macro_rules! impl_dependancy {
    ($($t:ident),*) => {
        impl< $($t: Send + Sync + Component),* > Dependancy for ($($t,)*) {
            type ElementBorrowed<'a> = ($(View<'a, $t>,)*);

            fn type_ids() -> Box<[TypeId]> {
                Box::new([$(TypeId::of::<$t>()),*])
            }
        }
    };
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

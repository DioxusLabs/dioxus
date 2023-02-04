use anymap::AnyMap;
use core::panic;
use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHashSet};
use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::node::{FromAnyValue, NodeType};
use crate::node_ref::{NodeMaskBuilder, NodeView};
use crate::real_dom::RealDom;
use crate::tree::{SlabEntry, Tree, TreeStateView};
use crate::{FxDashSet, SendAnyMap};
use crate::{NodeId, NodeMask};

#[derive(Default)]
struct DirtyNodes {
    passes_dirty: Vec<u64>,
}

impl DirtyNodes {
    pub fn add_node(&mut self, node_id: NodeId) {
        let node_id = node_id.0;
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

    pub fn pop(&mut self) -> Option<NodeId> {
        let index = self.passes_dirty.iter().position(|dirty| *dirty != 0)?;
        let passes = self.passes_dirty[index];
        let node_id = passes.trailing_zeros();
        let encoded = 1 << node_id;
        self.passes_dirty[index] &= !encoded;
        Some(NodeId((index * 64) + node_id as usize))
    }
}

#[derive(Clone)]
pub struct DirtyNodeStates {
    dirty: Arc<FxHashMap<TypeId, RwLock<BTreeMap<u16, DirtyNodes>>>>,
}

impl DirtyNodeStates {
    pub fn with_passes(passes: impl Iterator<Item = TypeId>) -> Self {
        let mut dirty = FxHashMap::default();
        for pass in passes {
            dirty.insert(pass, RwLock::new(BTreeMap::new()));
        }
        Self {
            dirty: Arc::new(dirty),
        }
    }

    pub fn insert(&self, pass_id: TypeId, node_id: NodeId, height: u16) {
        let btree = self.dirty.get(&pass_id).unwrap();
        let mut write = btree.write();
        if let Some(entry) = write.get_mut(&height) {
            entry.add_node(node_id);
        } else {
            let mut entry = DirtyNodes::default();
            entry.add_node(node_id);
            write.insert(height, entry);
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

pub trait Pass<V: FromAnyValue + Send + Sync = ()>: Any + Send + Sync {
    /// This is a tuple of (T: Any, ..)
    type ParentDependencies: Dependancy;
    /// This is a tuple of (T: Any, ..)
    type ChildDependencies: Dependancy;
    /// This is a tuple of (T: Any, ..)
    type NodeDependencies: Dependancy;
    const NODE_MASK: NodeMaskBuilder;

    fn pass<'a>(
        &mut self,
        node_view: NodeView<V>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
        context: &SendAnyMap,
    ) -> bool;

    fn create<'a>(
        node_view: NodeView<V>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
        context: &SendAnyMap,
    ) -> Self;

    fn validate() {
        // this type should not be a node dependency
        for type_id in Self::node_type_ids().iter().copied() {
            if type_id == TypeId::of::<Self>() {
                panic!("The current type cannot be a node dependency");
            }
        }
        // this type cannot be both a parent and child dependency
        assert!(
            Self::parent_type_ids()
                .iter()
                .any(|type_id| *type_id != TypeId::of::<Self>())
                && Self::child_type_ids()
                    .iter()
                    .any(|type_id| *type_id != TypeId::of::<Self>()),
            "The current type cannot be a parent and child dependency"
        );
        // no states have the same type id
        if Self::child_type_ids()
            .iter()
            .collect::<FxHashSet<_>>()
            .len()
            != Self::child_type_ids().len()
            || Self::parent_type_ids()
                .iter()
                .collect::<FxHashSet<_>>()
                .len()
                != Self::parent_type_ids().len()
            || Self::node_type_ids().iter().collect::<FxHashSet<_>>().len()
                != Self::node_type_ids().len()
        {
            panic!("all states must have unique type ids");
        }
    }

    fn to_type_erased() -> TypeErasedPass<V>
    where
        Self: Sized,
    {
        Self::validate();
        let node_mask = Self::NODE_MASK.build();
        TypeErasedPass {
            this_type_id: TypeId::of::<Self>(),
            combined_dependancy_type_ids: Self::all_dependanices().iter().copied().collect(),
            parent_dependant: !Self::parent_type_ids().is_empty(),
            child_dependant: !Self::child_type_ids().is_empty(),
            dependants: FxHashSet::default(),
            mask: node_mask.clone(),
            pass_direction: Self::pass_direction(),
            pass: Box::new(
                move |node_id: NodeId, tree: &mut TreeStateView, context: &SendAnyMap| {
                    debug_assert!(!Self::NodeDependencies::type_ids()
                        .iter()
                        .any(|id| *id == TypeId::of::<Self>()));
                    // get all of the states from the tree view
                    // Safety: No node has itself as a parent or child.
                    let myself: SlabEntry<'static, Self> = unsafe {
                        std::mem::transmute(tree.get_slab_mut::<Self>().unwrap().entry(node_id))
                    };
                    let node_data = tree.get_single::<NodeType<V>>(node_id).unwrap();
                    let node = tree.get::<Self::NodeDependencies>(node_id).unwrap();
                    let children = tree.children::<Self::ChildDependencies>(node_id);
                    let parent = tree.parent::<Self::ParentDependencies>(node_id);

                    let view = NodeView::new(node_id, node_data, &node_mask);
                    if myself.value.is_none() {
                        *myself.value = Some(Self::create(view, node, parent, children, context));
                        true
                    } else {
                        myself
                            .value
                            .as_mut()
                            .unwrap()
                            .pass(view, node, parent, children, context)
                    }
                },
            ) as PassCallback,
            create: Box::new(|tree: &mut Tree| tree.insert_slab::<Self>()) as CreatePassCallback,
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
    pass: PassCallback,
    pub(crate) create: CreatePassCallback,
    pub(crate) pass_direction: PassDirection,
    phantom: PhantomData<V>,
}

impl<V: FromAnyValue + Send> TypeErasedPass<V> {
    fn resolve(
        &self,
        mut tree: TreeStateView,
        dirty: &DirtyNodeStates,
        nodes_updated: &FxDashSet<NodeId>,
        ctx: &SendAnyMap,
    ) {
        match self.pass_direction {
            PassDirection::ParentToChild => {
                while let Some((height, id)) = dirty.pop_front(self.this_type_id) {
                    if (self.pass)(id, &mut tree, ctx) {
                        nodes_updated.insert(id);
                        for id in tree.children_ids(id).unwrap() {
                            for dependant in &self.dependants {
                                dirty.insert(*dependant, *id, height + 1);
                            }
                        }
                    }
                }
            }
            PassDirection::ChildToParent => {
                while let Some((height, id)) = dirty.pop_back(self.this_type_id) {
                    if (self.pass)(id, &mut tree, ctx) {
                        nodes_updated.insert(id);
                        if let Some(id) = tree.parent_id(id) {
                            for dependant in &self.dependants {
                                dirty.insert(*dependant, id, height - 1);
                            }
                        }
                    }
                }
            }
            PassDirection::AnyOrder => {
                while let Some((height, id)) = dirty.pop_back(self.this_type_id) {
                    if (self.pass)(id, &mut tree, ctx) {
                        nodes_updated.insert(id);
                        for dependant in &self.dependants {
                            dirty.insert(*dependant, id, height);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum PassDirection {
    ParentToChild,
    ChildToParent,
    AnyOrder,
}

type PassCallback = Box<dyn Fn(NodeId, &mut TreeStateView, &SendAnyMap) -> bool + Send + Sync>;
type CreatePassCallback = Box<dyn Fn(&mut Tree) + Send + Sync>;

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
    type ElementBorrowed<'a>
    where
        Self: 'a;

    fn borrow_elements_from<'a, T: AnyMapLike<'a> + Copy>(
        map: T,
    ) -> Option<Self::ElementBorrowed<'a>>;
    fn type_ids() -> Box<[TypeId]>;
}

macro_rules! impl_dependancy {
    ($($t:ident),*) => {
        impl< $($t: Any + Send + Sync),* > Dependancy for ($($t,)*) {
            type ElementBorrowed<'a> = ($(&'a $t,)*) where Self: 'a;

            #[allow(unused_variables, clippy::unused_unit, non_snake_case)]
            fn borrow_elements_from<'a, T: AnyMapLike<'a> + Copy>(map: T) -> Option<Self::ElementBorrowed<'a>> {
                Some(($(map.get::<$t>()?,)*))
            }

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
impl_dependancy!(A, B, C, D, E, F, G, H, I, J, K);
impl_dependancy!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_dependancy!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_dependancy!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_dependancy!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_dependancy!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

pub fn resolve_passes<V: FromAnyValue + Send + Sync>(
    tree: &mut RealDom<V>,
    dirty_nodes: DirtyNodeStates,
    ctx: SendAnyMap,
    parallel: bool,
) -> FxDashSet<NodeId> {
    let passes = &tree.passes;
    let mut resolved_passes: FxHashSet<TypeId> = FxHashSet::default();
    let mut resolving = Vec::new();
    let nodes_updated = Arc::new(FxDashSet::default());
    let ctx = Arc::new(ctx);
    let mut pass_indexes_remaining: Vec<_> = (0..passes.len()).collect::<Vec<_>>();
    while !pass_indexes_remaining.is_empty() {
        let mut currently_in_use = FxHashSet::<TypeId>::default();
        let dynamically_borrowed_tree = tree.tree.dynamically_borrowed();
        rayon::in_place_scope(|s| {
            let mut i = 0;
            while i < pass_indexes_remaining.len() {
                let passes_idx = pass_indexes_remaining[i];
                let pass = &passes[passes_idx];
                let pass_id = pass.this_type_id;
                // check if the pass is ready to be run
                if pass.combined_dependancy_type_ids.iter().all(|d| {
                    (resolved_passes.contains(d) || *d == pass_id) && !currently_in_use.contains(d)
                }) {
                    pass_indexes_remaining.remove(i);
                    resolving.push(pass_id);
                    currently_in_use.insert(pass.this_type_id);
                    let tree_view = dynamically_borrowed_tree.view(
                        pass.combined_dependancy_type_ids
                            .iter()
                            .filter(|id| **id != pass.this_type_id)
                            .copied()
                            .chain(std::iter::once(TypeId::of::<NodeType<V>>())),
                        [pass.this_type_id],
                    );
                    let dirty_nodes = dirty_nodes.clone();
                    let nodes_updated = nodes_updated.clone();
                    let ctx = ctx.clone();
                    if parallel {
                        s.spawn(move |_| {
                            pass.resolve(tree_view, &dirty_nodes, &nodes_updated, &ctx);
                        });
                    } else {
                        pass.resolve(tree_view, &dirty_nodes, &nodes_updated, &ctx);
                    }
                } else {
                    i += 1;
                }
            }
        });
        resolved_passes.extend(resolving.iter().copied());
        resolving.clear()
    }
    std::sync::Arc::try_unwrap(nodes_updated).unwrap()
}

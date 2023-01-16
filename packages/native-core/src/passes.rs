use anymap::AnyMap;
use rustc_hash::FxHashSet;
use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::node::{FromAnyValue, Node};
use crate::node_ref::NodeView;
use crate::state::State;
use crate::tree::TreeViewMut;
use crate::tree::{Tree, TreeView};
use crate::{FxDashMap, FxDashSet, SendAnyMap};
use crate::{NodeId, NodeMask};

pub trait Pass<V: FromAnyValue = ()>: Any {
    /// This is a tuple of (T: Any, ..)
    type ParentDependencies: Dependancy;
    /// This is a tuple of (T: Any, ..)
    type ChildDependencies: Dependancy;
    /// This is a tuple of (T: Any, ..)
    type NodeDependencies: Dependancy;
    const NODE_MASK: NodeMask;

    fn pass<'a>(
        &mut self,
        node_view: NodeView<V>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Option<
            impl Iterator<Item = <Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        >,
        context: &SendAnyMap,
    ) -> bool;

    fn validate() {
        // no type can be a child and parent dependency
        for type_id in Self::parent_type_ids() {
            for type_id2 in Self::child_type_ids() {
                if type_id == type_id2 {
                    panic!("type cannot be both a parent and child dependency");
                }
            }
        }
        // this type should not be a node dependency
        for type_id in Self::node_type_ids() {
            if type_id == TypeId::of::<Self>() {
                panic!("The current type cannot be a node dependency");
            }
        }
        // no states have the same type id
        if Self::all_dependanices()
            .into_iter()
            .collect::<FxDashSet<_>>()
            .len()
            != Self::all_dependanices().len()
        {
            panic!("all states must have unique type ids");
        }
    }

    fn to_type_erased<T: AnyMapLike + State<V>>() -> TypeErasedPass<T, V>
    where
        Self: Sized,
    {
        Self::validate();
        TypeErasedPass {
            this_type_id: TypeId::of::<Self>(),
            combined_dependancy_type_ids: Self::all_dependanices().into_iter().collect(),
            parent_dependant: !Self::parent_type_ids().is_empty(),
            child_dependant: !Self::child_type_ids().is_empty(),
            dependants: FxHashSet::default(),
            mask: Self::NODE_MASK,
            pass_direction: Self::pass_direction(),
            pass: Box::new(
                |node_id: NodeId, any_map: &mut Tree<Node<T, V>>, context: &SendAnyMap| {
                    let (current_node, parent, children) = any_map
                        .node_parent_children_mut(node_id)
                        .expect("tried to run pass on node that does not exist");
                    let current_node_raw = current_node as *mut Node<T, V>;
                    let node = Self::NodeDependencies::borrow_elements_from(&current_node.state)
                        .expect("tried to get a pass that does not exist");
                    let parent = parent.map(|parent| {
                        Self::ParentDependencies::borrow_elements_from(&parent.state)
                            .expect("tried to get a pass that does not exist")
                    });
                    let children = children.map(|children| {
                        children.map(|child| {
                            Self::ChildDependencies::borrow_elements_from(&child.state)
                                .expect("tried to get a pass that does not exist")
                        })
                    });
                    // safety: we have varified the pass is valid in the is_valid function
                    let myself: &mut Self = unsafe {
                        (*current_node_raw)
                            .state
                            .get_mut()
                            .expect("tried to get a pass that does not exist")
                    };
                    myself.pass(
                        NodeView::new(&current_node.node_data, Self::NODE_MASK),
                        node,
                        parent,
                        children,
                        context,
                    )
                },
            )
                as Box<dyn Fn(NodeId, &mut Tree<Node<T, V>>, &SendAnyMap) -> bool + Send + Sync>,
        }
    }

    fn parent_type_ids() -> Vec<TypeId> {
        Self::ParentDependencies::type_ids()
    }

    fn child_type_ids() -> Vec<TypeId> {
        Self::ChildDependencies::type_ids()
    }

    fn node_type_ids() -> Vec<TypeId> {
        Self::NodeDependencies::type_ids()
    }

    fn all_dependanices() -> Vec<TypeId> {
        let mut dependencies = Self::parent_type_ids();
        dependencies.extend(Self::child_type_ids());
        dependencies.extend(Self::node_type_ids());
        dependencies
    }

    fn pass_direction() -> PassDirection {
        if Self::child_type_ids()
            .into_iter()
            .any(|type_id| type_id == TypeId::of::<Self>())
        {
            PassDirection::ChildToParent
        } else if Self::parent_type_ids()
            .into_iter()
            .any(|type_id| type_id == TypeId::of::<Self>())
        {
            PassDirection::ParentToChild
        } else {
            PassDirection::AnyOrder
        }
    }
}

pub struct TypeErasedPass<T: AnyMapLike + State<V>, V: FromAnyValue = ()> {
    pub(crate) this_type_id: TypeId,
    pub(crate) parent_dependant: bool,
    pub(crate) child_dependant: bool,
    pub(crate) combined_dependancy_type_ids: FxHashSet<TypeId>,
    pub(crate) dependants: FxHashSet<TypeId>,
    pub(crate) mask: NodeMask,
    pass: PassCallback<T, V>,
    pub(crate) pass_direction: PassDirection,
}

impl<T: AnyMapLike + State<V>, V: FromAnyValue> TypeErasedPass<T, V> {
    fn resolve(
        &self,
        tree: &mut Tree<Node<T, V>>,
        mut dirty: DirtyNodes,
        dirty_states: &DirtyNodeStates,
        nodes_updated: &FxDashSet<NodeId>,
        ctx: &SendAnyMap,
    ) {
        match self.pass_direction {
            PassDirection::ParentToChild => {
                while let Some(id) = dirty.pop_front() {
                    if (self.pass)(id, tree, ctx) {
                        nodes_updated.insert(id);
                        for id in tree.children_ids(id).unwrap() {
                            for dependant in &self.dependants {
                                dirty_states.insert(*dependant, *id);
                            }

                            let height = tree.height(*id).unwrap();
                            dirty.insert(height, *id);
                        }
                    }
                }
            }
            PassDirection::ChildToParent => {
                while let Some(id) = dirty.pop_back() {
                    if (self.pass)(id, tree, ctx) {
                        nodes_updated.insert(id);
                        if let Some(id) = tree.parent_id(id) {
                            for dependant in &self.dependants {
                                dirty_states.insert(*dependant, id);
                            }

                            let height = tree.height(id).unwrap();
                            dirty.insert(height, id);
                        }
                    }
                }
            }
            PassDirection::AnyOrder => {
                while let Some(id) = dirty.pop_back() {
                    if (self.pass)(id, tree, ctx) {
                        nodes_updated.insert(id);
                        for dependant in &self.dependants {
                            dirty_states.insert(*dependant, id);
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

type PassCallback<T, V> =
    Box<dyn Fn(NodeId, &mut Tree<Node<T, V>>, &SendAnyMap) -> bool + Send + Sync>;

pub trait AnyMapLike {
    fn get<T: Any>(&self) -> Option<&T>;
    fn get_mut<T: Any>(&mut self) -> Option<&mut T>;
}

impl AnyMapLike for AnyMap {
    fn get<T: Any>(&self) -> Option<&T> {
        self.get()
    }

    fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.get_mut()
    }
}

impl AnyMapLike for SendAnyMap {
    fn get<T: Any>(&self) -> Option<&T> {
        todo!()
    }

    fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        todo!()
    }
}

pub trait Dependancy {
    type ElementBorrowed<'a>
    where
        Self: 'a;

    fn borrow_elements_from<T: AnyMapLike>(map: &T) -> Option<Self::ElementBorrowed<'_>>;
    fn type_ids() -> Vec<TypeId>;
}

macro_rules! impl_dependancy {
    ($($t:ident),*) => {
        impl< $($t: Any),* > Dependancy for ($($t,)*) {
            type ElementBorrowed<'a> = ($(&'a $t,)*) where Self: 'a;

            #[allow(unused_variables, clippy::unused_unit, non_snake_case)]
            fn borrow_elements_from<T: AnyMapLike>(map: &T) -> Option<Self::ElementBorrowed<'_>> {
                Some(($(map.get::<$t>()?,)*))
            }

            fn type_ids() -> Vec<TypeId> {
                vec![$(TypeId::of::<$t>()),*]
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirtyNodes {
    map: BTreeMap<u16, FxHashSet<NodeId>>,
}

impl DirtyNodes {
    fn add_node(&mut self, node_id: NodeId) {
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

    fn is_empty(&self) -> bool {
        self.passes_dirty.iter().all(|dirty| *dirty == 0)
    }

    fn pop(&mut self) -> Option<NodeId> {
        let index = self.passes_dirty.iter().position(|dirty| *dirty != 0)?;
        let passes = self.passes_dirty[index];
        let node_id = passes.trailing_zeros();
        let encoded = 1 << node_id;
        self.passes_dirty[index] &= !encoded;
        Some(NodeId((index * 64) + node_id as usize))
    }
}

#[derive(Default)]
pub struct DirtyNodeStates {
    dirty: FxDashMap<NodeId, FxHashSet<TypeId>>,
}

impl DirtyNodeStates {
    pub fn insert(&self, pass_id: TypeId, node_id: NodeId) {
        if let Some(mut dirty) = self.dirty.get_mut(&node_id) {
            dirty.insert(pass_id);
        } else {
            let mut v = FxHashSet::default();
            v.insert(pass_id);
            self.dirty.insert(node_id, v);
        }
    }

    fn all_dirty<T>(&self, pass_id: TypeId, dirty_nodes: &mut DirtyNodes, tree: &impl TreeView<T>) {
        for entry in self.dirty.iter() {
            let node_id = entry.key();
            let dirty = entry.value();
            if dirty.contains(&pass_id) {
                dirty_nodes.insert(tree.height(*node_id).unwrap(), *node_id);
            }
        }
        if values.is_empty() {
            self.dirty.remove(&height);
        }

        Some((height, node_id))
    }

    fn pop_back(&mut self, pass_id: PassId) -> Option<(u16, NodeId)> {
        let (&height, values) = self
            .dirty
            .iter_mut()
            .rev()
            .find(|(_, values)| values.contains_key(&pass_id))?;
        let dirty = values.get_mut(&pass_id)?;
        let node_id = dirty.pop()?;
        if dirty.is_empty() {
            values.remove(&pass_id);
        }
        if values.is_empty() {
            self.dirty.remove(&height);
        }

        Some((height, node_id))
    }
}

pub fn resolve_passes<T: AnyMapLike + State<V> + Send, V: FromAnyValue + Send>(
    tree: &mut Tree<Node<T, V>>,
    dirty_nodes: DirtyNodeStates,
    passes: &[TypeErasedPass<T, V>],
    ctx: SendAnyMap,
) -> FxDashSet<NodeId> {
    let dirty_states = Arc::new(dirty_nodes);
    let mut resolved_passes: FxHashSet<TypeId> = FxHashSet::default();
    let mut resolving = Vec::new();
    let nodes_updated = Arc::new(FxDashSet::default());
    let ctx = Arc::new(ctx);
    let mut pass_indexes_remaining: Vec<_> = (0..passes.len()).collect::<Vec<_>>();
    while !pass_indexes_remaining.is_empty() {
        let mut currently_in_use = FxHashSet::<TypeId>::default();
        std::thread::scope(|s| {
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
                    // this is safe because each pass only has mutable access to the member associated with this_type_id and we have verified that the pass does not overlap with any other pass currently running
                    let tree_unbounded_mut = unsafe { &mut *(tree as *mut _) };
                    let dirty_states = dirty_states.clone();
                    let nodes_updated = nodes_updated.clone();
                    let ctx = ctx.clone();
                    s.spawn(move || {
                        let mut dirty = DirtyNodes::default();
                        dirty_states.all_dirty(pass_id, &mut dirty, tree_unbounded_mut);
                        pass.resolve(
                            tree_unbounded_mut,
                            dirty,
                            &dirty_states,
                            &nodes_updated,
                            &ctx,
                        );
                    });
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

// #[test]
// fn node_pass() {
//     use crate::real_dom::RealDom;
//     use crate::tree::{Tree, TreeLike};

//     #[derive(Debug, Default, Clone, PartialEq)]
//     struct Number(i32);

//     impl State for Number {
//         fn create_passes() -> Box<[TypeErasedPass<Self>]> {
//             Box::new([Number::to_type_erased()])
//         }
//     }

//     impl AnyMapLike for Number {
//         fn get<T: Any>(&self) -> Option<&T> {
//             if TypeId::of::<Self>() == TypeId::of::<T>() {
//                 Some(unsafe { &*(self as *const Self as *const T) })
//             } else {
//                 None
//             }
//         }

//         fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
//             if TypeId::of::<Self>() == TypeId::of::<T>() {
//                 Some(unsafe { &mut *(self as *mut Self as *mut T) })
//             } else {
//                 None
//             }
//         }
//     }

//     impl Pass for Number {
//         type ChildDependencies = ();
//         type NodeDependencies = ();
//         type ParentDependencies = ();
//         type Ctx = ();
//         const MASK: NodeMask = NodeMask::new();

//         fn pass<'a>(
//             &mut self,
//             node_view: NodeView,
//             node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
//             parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
//             children: Option<
//                 impl Iterator<Item = <Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
//             >,
//             context: <Self::Ctx as Dependancy>::ElementBorrowed<'a>,
//         ) -> bool {
//             self.0 += 1;
//             true
//         }
//     }

//     let mut tree: RealDom<Number> = RealDom::new();
//     tree.dirty_nodes.insert(TypeId::of::<Number>(), NodeId(0));
//     tree.update_state(SendAnyMap::new());

//     assert_eq!(tree.get(tree.root()).unwrap().state.0, 1);
// }

// #[test]
// fn dependant_node_pass() {
//     use crate::real_dom::RealDom;

//     #[derive(Debug, Default, Clone, PartialEq)]
//     struct AddNumber(i32);

//     impl Pass for AddNumber {
//         type ChildDependencies = ();
//         type NodeDependencies = (SubtractNumber,);
//         type ParentDependencies = ();
//         type Ctx = ();
//         const MASK: NodeMask = NodeMask::new();

//         fn pass<'a>(
//             &mut self,
//             node_view: NodeView,
//             node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
//             parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
//             children: Option<
//                 impl Iterator<Item = <Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
//             >,
//             context: <Self::Ctx as Dependancy>::ElementBorrowed<'a>,
//         ) -> bool {
//             self.0 += 1;
//             true
//         }
//     }

//     #[derive(Debug, Default, Clone, PartialEq)]
//     struct SubtractNumber(i32);

//     impl Pass for SubtractNumber {
//         type ChildDependencies = ();
//         type NodeDependencies = ();
//         type ParentDependencies = ();
//         type Ctx = ();
//         const MASK: NodeMask = NodeMask::new();

//         fn pass<'a>(
//             &mut self,
//             node_view: NodeView,
//             node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
//             parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
//             children: Option<
//                 impl Iterator<Item = <Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
//             >,
//             context: <Self::Ctx as Dependancy>::ElementBorrowed<'a>,
//         ) -> bool {
//             self.0 -= 1;
//             true
//         }
//     }

//     #[derive(Debug, Default, Clone, PartialEq)]
//     struct NumberState {
//         add_number: AddNumber,
//         subtract_number: SubtractNumber,
//     }

//     impl AnyMapLike for NumberState {
//         fn get<T: Any>(&self) -> Option<&T> {
//             if TypeId::of::<AddNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &*(&self.add_number as *const AddNumber as *const T) })
//             } else if TypeId::of::<SubtractNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &*(&self.subtract_number as *const SubtractNumber as *const T) })
//             } else {
//                 None
//             }
//         }

//         fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
//             if TypeId::of::<AddNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &mut *(&mut self.add_number as *mut AddNumber as *mut T) })
//             } else if TypeId::of::<SubtractNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &mut *(&mut self.subtract_number as *mut SubtractNumber as *mut T) })
//             } else {
//                 None
//             }
//         }
//     }

//     impl State for NumberState {
//         fn create_passes() -> Box<[TypeErasedPass<Self>]> {
//             Box::new([
//                 AddNumber::to_type_erased(),
//                 SubtractNumber::to_type_erased(),
//             ])
//         }
//     }

//     let mut tree: RealDom<NumberState> = RealDom::new();
//     tree.dirty_nodes
//         .insert(TypeId::of::<SubtractNumber>(), NodeId(0));
//     tree.update_state(dirty_nodes, SendAnyMap::new());

//     assert_eq!(
//         tree.get(tree.root()).unwrap().state,
//         NumberState {
//             add_number: AddNumber(1),
//             subtract_number: SubtractNumber(-1)
//         }
//     );
// }

// #[test]
// fn independant_node_pass() {
//     use crate::real_dom::RealDom;
//     use crate::tree::{Tree, TreeLike};

//     #[derive(Debug, Default, Clone, PartialEq)]
//     struct AddNumber(i32);

//     impl Pass for AddNumber {
//         type ChildDependencies = ();
//         type NodeDependencies = ();
//         type ParentDependencies = ();
//         type Ctx = ();
//         const MASK: NodeMask = NodeMask::new();

//         fn pass<'a>(
//             &mut self,
//             node_view: NodeView,
//             node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
//             parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
//             children: Option<
//                 impl Iterator<Item = <Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
//             >,
//             context: <Self::Ctx as Dependancy>::ElementBorrowed<'a>,
//         ) -> bool {
//             self.0 += 1;
//             true
//         }
//     }

//     #[derive(Debug, Default, Clone, PartialEq)]
//     struct SubtractNumber(i32);

//     impl Pass for SubtractNumber {
//         type ChildDependencies = ();
//         type NodeDependencies = ();
//         type ParentDependencies = ();
//         type Ctx = ();
//         const MASK: NodeMask = NodeMask::new();

//         fn pass<'a>(
//             &mut self,
//             node_view: NodeView,
//             node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
//             parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
//             children: Option<
//                 impl Iterator<Item = <Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
//             >,
//             context: <Self::Ctx as Dependancy>::ElementBorrowed<'a>,
//         ) -> bool {
//             self.0 -= 1;
//             true
//         }
//     }

//     #[derive(Debug, Default, Clone, PartialEq)]
//     struct NumberState {
//         add_number: AddNumber,
//         subtract_number: SubtractNumber,
//     }

//     impl AnyMapLike for NumberState {
//         fn get<T: Any>(&self) -> Option<&T> {
//             if TypeId::of::<AddNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &*(&self.add_number as *const AddNumber as *const T) })
//             } else if TypeId::of::<SubtractNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &*(&self.subtract_number as *const SubtractNumber as *const T) })
//             } else {
//                 None
//             }
//         }

//         fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
//             if TypeId::of::<AddNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &mut *(&mut self.add_number as *mut AddNumber as *mut T) })
//             } else if TypeId::of::<SubtractNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &mut *(&mut self.subtract_number as *mut SubtractNumber as *mut T) })
//             } else {
//                 None
//             }
//         }
//     }

//     impl State for NumberState {
//         fn create_passes() -> Box<[TypeErasedPass<Self>]> {
//             Box::new([
//                 AddNumber::to_type_erased(),
//                 SubtractNumber::to_type_erased(),
//             ])
//         }
//     }

//     let mut tree: RealDom<NumberState> = RealDom::new();
//     tree.dirty_nodes
//         .insert(TypeId::of::<SubtractNumber>(), NodeId(0));
//     tree.update_state(SendAnyMap::new());

//     assert_eq!(
//         tree.get(tree.root()).unwrap().state,
//         NumberState {
//             add_number: AddNumber(0),
//             subtract_number: SubtractNumber(-1)
//         }
//     );
// }

// #[test]
// fn down_pass() {
//     use crate::real_dom::RealDom;
//     use crate::tree::{Tree, TreeLike};

//     #[derive(Debug, Default, Clone, PartialEq)]
//     struct AddNumber(i32);

//     impl Pass for AddNumber {
//         type ChildDependencies = ();
//         type NodeDependencies = ();
//         type ParentDependencies = (AddNumber,);
//         type Ctx = ();
//         const MASK: NodeMask = NodeMask::new();

//         fn pass<'a>(
//             &mut self,
//             node_view: NodeView,
//             node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
//             parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
//             children: Option<
//                 impl Iterator<Item = <Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
//             >,
//             context: <Self::Ctx as Dependancy>::ElementBorrowed<'a>,
//         ) -> bool {
//             if let Some((parent,)) = parent {
//                 *self.0 += *parent.0;
//             }
//             true
//         }
//     }

//     #[derive(Debug, Default, Clone, PartialEq)]
//     struct NumberState {
//         add_number: AddNumber,
//     }

//     impl AnyMapLike for NumberState {
//         fn get<T: Any>(&self) -> Option<&T> {
//             if TypeId::of::<AddNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &*(&self.add_number as *const AddNumber as *const T) })
//             } else {
//                 None
//             }
//         }

//         fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
//             if TypeId::of::<AddNumber>() == TypeId::of::<T>() {
//                 Some(unsafe { &mut *(&mut self.add_number as *mut AddNumber as *mut T) })
//             } else {
//                 None
//             }
//         }
//     }

//     impl State for NumberState {
//         fn create_passes() -> Box<[TypeErasedPass<Self>]> {
//             Box::new([AddNumber::to_type_erased()])
//         }
//     }

//     let mut tree: RealDom<NumberState> = RealDom::new();
//     let parent = tree.root();
//     let child1 = tree.create_node(1);
//     tree.add_child(parent, child1);
//     let grandchild1 = tree.create_node(1);
//     tree.add_child(child1, grandchild1);
//     let child2 = tree.create_node(1);
//     tree.add_child(parent, child2);
//     let grandchild2 = tree.create_node(1);
//     tree.add_child(child2, grandchild2);

//     tree.dirty_nodes
//         .insert(TypeId::of::<AddNumber>(), NodeId(0));
//     tree.update_state(SendAnyMap::new());

//     assert_eq!(tree.get(tree.root()).unwrap().state.add_number.0, 1);
//     assert_eq!(tree.get(child1).unwrap().state.add_number.0, 2);
//     assert_eq!(tree.get(grandchild1).unwrap().state.add_number.0, 3);
//     assert_eq!(tree.get(child2).unwrap().state.add_number.0, 2);
//     assert_eq!(tree.get(grandchild2).unwrap().state.add_number.0, 3);
// }

// #[test]
// fn dependant_down_pass() {
//     use crate::tree::{Tree, TreeLike};
//     // 0
//     let mut tree = Tree::new(1);
//     let parent = tree.root();
//     // 1
//     let child1 = tree.create_node(1);
//     tree.add_child(parent, child1);
//     // 2
//     let grandchild1 = tree.create_node(1);
//     tree.add_child(child1, grandchild1);
//     // 3
//     let child2 = tree.create_node(1);
//     tree.add_child(parent, child2);
//     // 4
//     let grandchild2 = tree.create_node(1);
//     tree.add_child(child2, grandchild2);

//     struct AddPass;
//     impl Pass for AddPass {
//         fn pass_id(&self) -> PassId {
//             PassId(0)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[PassId(1)]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl DownwardPass<i32> for AddPass {
//         fn pass(&self, node: &mut i32, parent: Option<&mut i32>, _: &SendAnyMap) -> PassReturn {
//             if let Some(parent) = parent {
//                 *node += *parent;
//             } else {
//             }
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     struct SubtractPass;
//     impl Pass for SubtractPass {
//         fn pass_id(&self) -> PassId {
//             PassId(1)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[PassId(0)]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl DownwardPass<i32> for SubtractPass {
//         fn pass(&self, node: &mut i32, parent: Option<&mut i32>, _: &SendAnyMap) -> PassReturn {
//             if let Some(parent) = parent {
//                 *node -= *parent;
//             } else {
//             }
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     let add_pass = AnyPass::Downward(&AddPass);
//     let subtract_pass = AnyPass::Downward(&SubtractPass);
//     let passes = vec![&add_pass, &subtract_pass];
//     let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
//     dirty_nodes.insert(PassId(1), tree.root());
//     resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

//     // Tree before:
//     // 1=\
//     //   1=\
//     //     1
//     //   1=\
//     //     1
//     // Tree after subtract:
//     // 1=\
//     //   0=\
//     //     1
//     //   0=\
//     //     1
//     // Tree after add:
//     // 1=\
//     //   1=\
//     //     2
//     //   1=\
//     //     2
//     assert_eq!(tree.get(tree.root()).unwrap(), &1);
//     assert_eq!(tree.get(child1).unwrap(), &1);
//     assert_eq!(tree.get(grandchild1).unwrap(), &2);
//     assert_eq!(tree.get(child2).unwrap(), &1);
//     assert_eq!(tree.get(grandchild2).unwrap(), &2);
// }

// #[test]
// fn up_pass() {
//     use crate::tree::{Tree, TreeLike};
//     // Tree before:
//     // 0=\
//     //   0=\
//     //     1
//     //   0=\
//     //     1
//     // Tree after:
//     // 2=\
//     //   1=\
//     //     1
//     //   1=\
//     //     1
//     let mut tree = Tree::new(0);
//     let parent = tree.root();
//     let child1 = tree.create_node(0);
//     tree.add_child(parent, child1);
//     let grandchild1 = tree.create_node(1);
//     tree.add_child(child1, grandchild1);
//     let child2 = tree.create_node(0);
//     tree.add_child(parent, child2);
//     let grandchild2 = tree.create_node(1);
//     tree.add_child(child2, grandchild2);

//     struct AddPass;
//     impl Pass for AddPass {
//         fn pass_id(&self) -> PassId {
//             PassId(0)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl UpwardPass<i32> for AddPass {
//         fn pass<'a>(
//             &self,
//             node: &mut i32,
//             children: &mut dyn Iterator<Item = &'a mut i32>,
//             _: &SendAnyMap,
//         ) -> PassReturn {
//             *node += children.map(|i| *i).sum::<i32>();
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     let add_pass = AnyPass::Upward(&AddPass);
//     let passes = vec![&add_pass];
//     let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
//     dirty_nodes.insert(PassId(0), grandchild1);
//     dirty_nodes.insert(PassId(0), grandchild2);
//     resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

//     assert_eq!(tree.get(tree.root()).unwrap(), &2);
//     assert_eq!(tree.get(child1).unwrap(), &1);
//     assert_eq!(tree.get(grandchild1).unwrap(), &1);
//     assert_eq!(tree.get(child2).unwrap(), &1);
//     assert_eq!(tree.get(grandchild2).unwrap(), &1);
// }

// #[test]
// fn dependant_up_pass() {
//     use crate::tree::{Tree, TreeLike};
//     // 0
//     let mut tree = Tree::new(0);
//     let parent = tree.root();
//     // 1
//     let child1 = tree.create_node(0);
//     tree.add_child(parent, child1);
//     // 2
//     let grandchild1 = tree.create_node(1);
//     tree.add_child(child1, grandchild1);
//     // 3
//     let child2 = tree.create_node(0);
//     tree.add_child(parent, child2);
//     // 4
//     let grandchild2 = tree.create_node(1);
//     tree.add_child(child2, grandchild2);

//     struct AddPass;
//     impl Pass for AddPass {
//         fn pass_id(&self) -> PassId {
//             PassId(0)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[PassId(1)]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl UpwardPass<i32> for AddPass {
//         fn pass<'a>(
//             &self,
//             node: &mut i32,
//             children: &mut dyn Iterator<Item = &'a mut i32>,
//             _: &SendAnyMap,
//         ) -> PassReturn {
//             *node += children.map(|i| *i).sum::<i32>();
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     struct SubtractPass;
//     impl Pass for SubtractPass {
//         fn pass_id(&self) -> PassId {
//             PassId(1)
//         }

//         fn dependancies(&self) -> &'static [PassId] {
//             &[]
//         }

//         fn dependants(&self) -> &'static [PassId] {
//             &[PassId(0)]
//         }

//         fn mask(&self) -> MemberMask {
//             MemberMask(0)
//         }
//     }
//     impl UpwardPass<i32> for SubtractPass {
//         fn pass<'a>(
//             &self,
//             node: &mut i32,
//             children: &mut dyn Iterator<Item = &'a mut i32>,
//             _: &SendAnyMap,
//         ) -> PassReturn {
//             *node -= children.map(|i| *i).sum::<i32>();
//             PassReturn {
//                 progress: true,
//                 mark_dirty: true,
//             }
//         }
//     }

//     let add_pass = AnyPass::Upward(&AddPass);
//     let subtract_pass = AnyPass::Upward(&SubtractPass);
//     let passes = vec![&add_pass, &subtract_pass];
//     let dirty_nodes: DirtyNodeStates = DirtyNodeStates::default();
//     dirty_nodes.insert(PassId(1), grandchild1);
//     dirty_nodes.insert(PassId(1), grandchild2);
//     resolve_passes(&mut tree, dirty_nodes, passes, SendAnyMap::new());

//     // Tree before:
//     // 0=\
//     //   0=\
//     //     1
//     //   0=\
//     //     1
//     // Tree after subtract:
//     // 2=\
//     //   -1=\
//     //      1
//     //   -1=\
//     //      1
//     // Tree after add:
//     // 2=\
//     //   0=\
//     //     1
//     //   0=\
//     //     1
//     assert_eq!(tree.get(tree.root()).unwrap(), &2);
//     assert_eq!(tree.get(child1).unwrap(), &0);
//     assert_eq!(tree.get(grandchild1).unwrap(), &1);
//     assert_eq!(tree.get(child2).unwrap(), &0);
//     assert_eq!(tree.get(grandchild2).unwrap(), &1);
// }

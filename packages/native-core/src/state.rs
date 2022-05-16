use std::{cmp::Ordering, fmt::Debug};

use anymap::AnyMap;
use dioxus_core::ElementId;
use fxhash::FxHashSet;

use crate::element_borrowable::ElementBorrowable;
use crate::node_ref::{NodeMask, NodeView};
use crate::traversable::Traversable;

pub(crate) fn union_ordered_iter<T: Ord + Debug>(
    s_iter: impl Iterator<Item = T>,
    o_iter: impl Iterator<Item = T>,
    new_len_guess: usize,
) -> Vec<T> {
    let mut s_peekable = s_iter.peekable();
    let mut o_peekable = o_iter.peekable();
    let mut v = Vec::with_capacity(new_len_guess);
    while let Some(s_i) = s_peekable.peek() {
        while let Some(o_i) = o_peekable.peek() {
            match o_i.cmp(s_i) {
                Ordering::Greater => {
                    break;
                }
                Ordering::Less => {
                    v.push(o_peekable.next().unwrap());
                }
                Ordering::Equal => {
                    o_peekable.next();
                    break;
                }
            }
        }
        v.push(s_peekable.next().unwrap());
    }
    for o_i in o_peekable {
        v.push(o_i);
    }
    for w in v.windows(2) {
        debug_assert!(w[1] > w[0]);
    }
    v
}

/// This state is derived from children. For example a node's size could be derived from the size of children.
/// Called when the current node's node properties are modified, a child's [BubbledUpState] is modified or a child is removed.
/// Called at most once per update.
pub trait ChildDepState {
    type Ctx;
    /// This must be either a [ChildDepState], or [NodeDepState]
    type DepState;
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        children: impl Iterator<Item = &'a Self::DepState>,
        ctx: &Self::Ctx,
    ) -> bool
    where
        Self::DepState: 'a;
}

/// This state that is passed down to children. For example text properties (`<b>` `<i>` `<u>`) would be passed to children.
/// Called when the current node's node properties are modified or a parrent's [PushedDownState] is modified.
/// Called at most once per update.
pub trait ParentDepState {
    type Ctx;
    /// This must be either a [ParentDepState] or [NodeDepState]
    type DepState;
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        parent: Option<&'a Self::DepState>,
        ctx: &Self::Ctx,
    ) -> bool;
}

/// This state that is upadated lazily. For example any propertys that do not effect other parts of the dom like bg-color.
/// Called when the current node's node properties are modified or a parrent's [PushedDownState] is modified.
/// Called at most once per update.
pub trait NodeDepState {
    type Ctx;
    /// This must be either a [ChildDepState], [ParentDepState] or [NodeDepState]
    type DepState: ElementBorrowable;
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        siblings: <Self::DepState as ElementBorrowable>::Borrowed<'a>,
        ctx: &Self::Ctx,
    ) -> bool;
}

pub trait State: Default + Clone {
    fn update<'a, T: Traversable<Node = Self, Id = ElementId>>(
        dirty: &Vec<(ElementId, NodeMask)>,
        state_tree: &'a mut T,
        vdom: &'a dioxus_core::VirtualDom,
        ctx: &AnyMap,
    ) -> FxHashSet<ElementId>;
}

// Todo: once GATs land we can model multable dependencies
impl ChildDepState for () {
    type Ctx = ();
    type DepState = ();
    fn reduce<'a>(
        &mut self,
        _: NodeView,
        _: impl Iterator<Item = &'a Self::DepState>,
        _: &Self::Ctx,
    ) -> bool
    where
        Self::DepState: 'a,
    {
        false
    }
}

impl ParentDepState for () {
    type Ctx = ();
    type DepState = ();
    fn reduce<'a>(&mut self, _: NodeView, _: Option<&'a Self::DepState>, _: &Self::Ctx) -> bool {
        false
    }
}

impl NodeDepState for () {
    type Ctx = ();
    type DepState = ();
    fn reduce(&mut self, _: NodeView, _sibling: Self::DepState, _: &Self::Ctx) -> bool {
        false
    }
}

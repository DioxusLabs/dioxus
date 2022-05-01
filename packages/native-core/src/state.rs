use std::{
    cmp::Ordering,
    fmt::Debug,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use anymap::AnyMap;
use dioxus_core::VNode;

use crate::element_borrowable::ElementBorrowable;
use crate::node_ref::{NodeMask, NodeView};

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
    /// This must be either a [ChildDepState], [ParentDepState] or [NodeDepState]
    type DepState: ElementBorrowable;
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        children: impl Iterator<Item = <Self::DepState as ElementBorrowable>::Borrowed<'a>>,
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
    /// This must be either a [ChildDepState], [ParentDepState] or [NodeDepState]
    type DepState: ElementBorrowable;
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        parent: Option<<Self::DepState as ElementBorrowable>::Borrowed<'a>>,
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

#[derive(Debug)]
pub struct StatesChanged {
    pub node_dep: Vec<MemberId>,
    pub parent_dep: Vec<MemberId>,
    pub child_dep: Vec<MemberId>,
}

pub trait State: Default + Clone {
    const SIZE: usize;

    fn update_dep_state<'a>(
        &'a mut self,
        ty: MemberId,
        node: &'a VNode<'a>,
        vdom: &'a dioxus_core::VirtualDom,
        ctx: &AnyMap,
    ) -> Option<StatesChanged>;
}

// Todo: once GATs land we can model multable dependencies
impl ChildDepState for () {
    type Ctx = ();
    type DepState = ();
    fn reduce<'a>(
        &mut self,
        _: NodeView,
        _: impl Iterator<Item = Self::DepState>,
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
    fn reduce(&mut self, _: NodeView, _: Option<Self::DepState>, _: &Self::Ctx) -> bool {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemberId(pub usize);

impl Sub<usize> for MemberId {
    type Output = MemberId;
    fn sub(self, rhs: usize) -> Self::Output {
        MemberId(self.0 - rhs)
    }
}

impl Add<usize> for MemberId {
    type Output = MemberId;
    fn add(self, rhs: usize) -> Self::Output {
        MemberId(self.0 + rhs)
    }
}

impl SubAssign<usize> for MemberId {
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs;
    }
}

impl AddAssign<usize> for MemberId {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

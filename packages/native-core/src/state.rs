use std::cmp::Ordering;

use crate::node::NodeData;
use crate::node_ref::{NodeMask, NodeView};
use crate::tree::TreeView;
use crate::RealNodeId;
use anymap::AnyMap;
use rustc_hash::FxHashSet;

/// Join two sorted iterators
pub(crate) fn union_ordered_iter<'a>(
    s_iter: impl Iterator<Item = &'a str>,
    o_iter: impl Iterator<Item = &'a str>,
    new_len_guess: usize,
) -> Vec<String> {
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
                    v.push(o_peekable.next().unwrap().to_string());
                }
                Ordering::Equal => {
                    o_peekable.next();
                    break;
                }
            }
        }
        v.push(s_peekable.next().unwrap().to_string());
    }
    for o_i in o_peekable {
        v.push(o_i.to_string());
    }
    for w in v.windows(2) {
        debug_assert!(w[1] > w[0]);
    }
    v
}

/// This state is derived from children. For example a node's size could be derived from the size of children.
/// Called when the current node's node properties are modified, a child's [ChildDepState] is modified or a child is removed.
/// Called at most once per update.
/// ```rust
/// # use dioxus_native_core::node_ref::NodeView;
/// # use dioxus_native_core::state::ChildDepState;
/// #[derive(Clone, Copy, PartialEq, Default)]
/// struct Layout {
///     width: u32,
///     height: u32,
/// }
///
/// impl ChildDepState for Layout {
///     type Ctx = ();
///     // The layout depends on the layout of the children.
///     type DepState = Layout;
///     fn reduce<'a>(
///         &mut self,
///         _node: NodeView,
///         children: impl Iterator<Item = &'a Self::DepState>,
///         _ctx: &Self::Ctx,
///     ) -> bool
///     where
///         Self::DepState: 'a{
///         /// Children are layed out form left to right. The width of the parent is the sum of the widths and the max of the heights.
///         let new = children.copied().reduce(|c1, c2| Layout{
///             width: c1.width + c2.width,
///             height: c1.height.max(c2.height)
///         }).unwrap_or_default();
///         let changed = new != *self;
///         *self = new;
///         changed
///     }
/// }
/// ```
pub trait ChildDepState {
    /// The context is passed to the [ChildDepState::reduce] when it is resolved.
    type Ctx;
    /// A state from each child node that this node depends on. Typically this is Self, but it could be any state that is within the state tree.
    /// This must be either a [ChildDepState], or [NodeDepState].
    type DepState;
    /// The part of a node that this state cares about. This is used to determine if the state should be updated when a node is updated.
    const NODE_MASK: NodeMask = NodeMask::NONE;
    /// Resolve the state current node's state from the state of the children, the state of the node, and some external context.
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
/// Called when the current node's node properties are modified or a parrent's [ParentDepState] is modified.
/// Called at most once per update.
/// ```rust
/// use dioxus_native_core::node_ref::{NodeMask, AttributeMask, NodeView};
/// use dioxus_native_core::state::*;
///
/// #[derive(Clone, Copy, PartialEq)]
/// struct FontSize(usize);
///
/// impl ParentDepState for FontSize {
///     type Ctx = ();
///     // The font size depends on the font size of the parent element.
///     type DepState = Self;
///     const NODE_MASK: NodeMask =
///         NodeMask::new_with_attrs(AttributeMask::Static(&[
///             "font-size"
///         ]));
///     fn reduce<'a>(
///         &mut self,
///         node: NodeView,
///         parent: Option<&'a Self::DepState>,
///         ctx: &Self::Ctx,
///     ) -> bool{
///         let old = *self;
///         // If the font size was set on the parent, it is passed down to the current element
///         if let Some(parent) = parent {
///             *self = *parent;
///         }
///         // If the current node overrides the font size, use that size insead.
///         for attr in node.attributes().unwrap() {
///             match attr.attribute.name.as_str() {
///                 "font-size" => {
///                     self.0 = attr.value.as_text().unwrap().parse().unwrap();
///                 }
///                 // font-size is the only attribute we specified in the mask, so it is the only one we can see
///                 _ => unreachable!(),
///             }
///         }
///         old != *self
///     }
/// }
/// ```
pub trait ParentDepState {
    /// The context is passed to the [ParentDepState::reduce] when it is resolved.
    type Ctx;
    /// A state from from the parent node that this node depends on. Typically this is Self, but it could be any state that is within the state tree.
    /// This must be either a [ParentDepState] or [NodeDepState]
    type DepState;
    /// The part of a node that this state cares about. This is used to determine if the state should be updated when a node is updated.
    const NODE_MASK: NodeMask = NodeMask::NONE;
    /// Resolve the state current node's state from the state of the parent node, the state of the node, and some external context.
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        parent: Option<&'a Self::DepState>,
        ctx: &Self::Ctx,
    ) -> bool;
}

/// This state that is upadated lazily. For example any propertys that do not effect other parts of the dom like bg-color.
/// Called when the current node's node properties are modified or a one of its dependanices are modified.
/// Called at most once per update.
/// NodeDepState is the only state that can accept multiple dependancies, but only from the current node.
/// ```rust
/// use dioxus_native_core::node_ref::{NodeMask, AttributeMask, NodeView};
/// use dioxus_native_core::state::*;
///
/// #[derive(Clone, Copy, PartialEq)]
/// struct TabIndex(usize);
///
/// impl NodeDepState for TabIndex {
///     type Ctx = ();
///     const NODE_MASK: NodeMask =
///         NodeMask::new_with_attrs(AttributeMask::Static(&[
///             "tabindex"
///         ]));
///     fn reduce(
///         &mut self,
///         node: NodeView,
///         siblings: (),
///         ctx: &(),
///     ) -> bool {
///         let old = self.clone();
///         for attr in node.attributes().unwrap() {
///             match attr.attribute.name.as_str() {
///                 "tabindex" => {
///                     self.0 = attr.value.as_text().unwrap().parse().unwrap();
///                 }
///                 // tabindex is the only attribute we specified in the mask, so it is the only one we can see
///                 _ => unreachable!(),
///             }
///         }
///         old != *self
///     }
/// }
/// ```
/// The generic argument (Depstate) must be a tuple containing any number of borrowed elements that are either a [ChildDepState], [ParentDepState] or [NodeDepState].
// Todo: once GATs land we can model multable dependencies better

pub trait NodeDepState<DepState = ()> {
    /// The state passed to [NodeDepState::reduce] when it is resolved.
    type Ctx;
    /// The part of a node that this state cares about. This is used to determine if the state should be updated when a node is updated.
    const NODE_MASK: NodeMask = NodeMask::NONE;
    /// Resolve the state current node's state from the state of the sibling states, the state of the node, and some external context.
    fn reduce(&mut self, node: NodeView, siblings: DepState, ctx: &Self::Ctx) -> bool;
}

/// Do not implement this trait. It is only meant to be derived and used through [crate::real_dom::RealDom].
pub trait State: Default + Clone {
    #[doc(hidden)]
    fn update<'a, T: TreeView<Self>, T2: TreeView<NodeData>>(
        dirty: &[(RealNodeId, NodeMask)],
        state_tree: &'a mut T,
        rdom: &'a T2,
        ctx: &AnyMap,
    ) -> FxHashSet<RealNodeId>;
}

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

impl NodeDepState<()> for () {
    type Ctx = ();
    fn reduce(&mut self, _: NodeView, _sibling: (), _: &Self::Ctx) -> bool {
        false
    }
}

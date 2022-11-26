use std::cmp::Ordering;

use crate::node::Node;
use crate::node_ref::{NodeMask, NodeView};
use crate::passes::{resolve_passes, AnyPass, DirtyNodeStates};
use crate::tree::TreeView;
use crate::{FxDashSet, RealNodeId, SendAnyMap};

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
    /// Depstate must be a tuple containing any number of borrowed elements that are either [ChildDepState] or [NodeDepState].
    type DepState: ElementBorrowable;
    /// The part of a node that this state cares about. This is used to determine if the state should be updated when a node is updated.
    const NODE_MASK: NodeMask = NodeMask::NONE;
    /// Resolve the state current node's state from the state of the children, the state of the node, and some external context.
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        children: impl Iterator<Item = <Self::DepState as ElementBorrowable>::ElementBorrowed<'a>>,
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
    /// Depstate must be a tuple containing any number of borrowed elements that are either [ParentDepState] or [NodeDepState].
    type DepState: ElementBorrowable;
    /// The part of a node that this state cares about. This is used to determine if the state should be updated when a node is updated.
    const NODE_MASK: NodeMask = NodeMask::NONE;
    /// Resolve the state current node's state from the state of the parent node, the state of the node, and some external context.
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        parent: Option<<Self::DepState as ElementBorrowable>::ElementBorrowed<'a>>,
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
pub trait NodeDepState {
    /// Depstate must be a tuple containing any number of borrowed elements that are either [ChildDepState], [ParentDepState] or [NodeDepState].
    type DepState: ElementBorrowable;
    /// The state passed to [NodeDepState::reduce] when it is resolved.
    type Ctx;
    /// The part of a node that this state cares about. This is used to determine if the state should be updated when a node is updated.
    const NODE_MASK: NodeMask = NodeMask::NONE;
    /// Resolve the state current node's state from the state of the sibling states, the state of the node, and some external context.
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        node_state: <Self::DepState as ElementBorrowable>::ElementBorrowed<'a>,
        ctx: &Self::Ctx,
    ) -> bool;
}

/// Do not implement this trait. It is only meant to be derived and used through [crate::real_dom::RealDom].
pub trait State: Default + Clone + 'static {
    #[doc(hidden)]
    const PASSES: &'static [AnyPass<Node<Self>>];
    #[doc(hidden)]
    const MASKS: &'static [NodeMask];

    #[doc(hidden)]
    fn update<T: TreeView<Node<Self>>>(
        dirty: DirtyNodeStates,
        tree: &mut T,
        ctx: SendAnyMap,
    ) -> FxDashSet<RealNodeId> {
        let set = FxDashSet::default();
        let passes = Self::PASSES.iter().collect();
        resolve_passes(tree, dirty, passes, ctx);
        set
    }
}

impl ChildDepState for () {
    type Ctx = ();
    type DepState = ();
    fn reduce<'a>(&mut self, _: NodeView, _: impl Iterator<Item = ()>, _: &Self::Ctx) -> bool
    where
        Self::DepState: 'a,
    {
        false
    }
}

impl ParentDepState for () {
    type Ctx = ();
    type DepState = ();
    fn reduce<'a>(&mut self, _: NodeView, _: Option<()>, _: &Self::Ctx) -> bool {
        false
    }
}

impl NodeDepState for () {
    type DepState = ();
    type Ctx = ();
    fn reduce(&mut self, _: NodeView, _sibling: (), _: &Self::Ctx) -> bool {
        false
    }
}

pub trait ElementBorrowable {
    type ElementBorrowed<'a>
    where
        Self: 'a;

    fn borrow_elements(&self) -> Self::ElementBorrowed<'_>;
}

macro_rules! impl_element_borrowable {
    ($($t:ident),*) => {
        impl< $($t),* > ElementBorrowable for ($($t,)*) {
            type ElementBorrowed<'a> = ($(&'a $t,)*) where Self: 'a;

            #[allow(clippy::unused_unit, non_snake_case)]
            fn borrow_elements<'a>(&'a self) -> Self::ElementBorrowed<'a> {
                let ($($t,)*) = self;
                ($(&$t,)*)
            }
        }
    };
}

impl_element_borrowable!();
impl_element_borrowable!(A);
impl_element_borrowable!(A, B);
impl_element_borrowable!(A, B, C);
impl_element_borrowable!(A, B, C, D);
impl_element_borrowable!(A, B, C, D, E);
impl_element_borrowable!(A, B, C, D, E, F);
impl_element_borrowable!(A, B, C, D, E, F, G);
impl_element_borrowable!(A, B, C, D, E, F, G, H);
impl_element_borrowable!(A, B, C, D, E, F, G, H, I);
impl_element_borrowable!(A, B, C, D, E, F, G, H, I, J);
impl_element_borrowable!(A, B, C, D, E, F, G, H, I, J, K);
impl_element_borrowable!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_element_borrowable!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_element_borrowable!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_element_borrowable!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_element_borrowable!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

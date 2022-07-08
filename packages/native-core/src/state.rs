use std::{cmp::Ordering, fmt::Debug};

use anymap::AnyMap;
use dioxus_core::ElementId;
use fxhash::FxHashSet;

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
/// Called when the current node's node properties are modified, a child's [ChildDepState] is modified or a child is removed.
/// Called at most once per update.
pub trait ChildDepState {
    /// The context is passed to the [ChildDepState::reduce] when it is pushed down.
    /// This is sometimes nessisary for lifetime purposes.
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
/// Called when the current node's node properties are modified or a parrent's [ParentDepState] is modified.
/// Called at most once per update.
pub trait ParentDepState {
    /// The context is passed to the [ParentDepState::reduce] when it is pushed down.
    /// This is sometimes nessisary for lifetime purposes.
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
/// Called when the current node's node properties are modified or a sibling's [NodeDepState] is modified.
/// Called at most once per update.
/// NodeDepState is the only state that can accept multiple dependancies, but only from the current node.
/// ```rust
/// impl<'a, 'b> NodeDepState<(&'a TextWrap, &'b ChildLayout)> for Layout {
///     type Ctx = LayoutCache;
///     const NODE_MASK: NodeMask =
///         NodeMask::new_with_attrs(AttributeMask::Static(&sorted_str_slice!([
///             "width", "height"
///         ])))
///         .with_text();
///     fn reduce<'a>(
///         &mut self,
///         node: NodeView,
///         siblings: (&'a TextWrap, &'b ChildLayout),
///         ctx: &Self::Ctx,
///     ) -> bool {
///         let old = self.clone();
///         let (text_wrap, child_layout) = siblings;
///         if TextWrap::Wrap == text_wrap {
///             if let Some(text) = node.text() {
///                 let lines = text_wrap.get_lines(text);
///                 self.width = lines.max_by(|l| l.len());
///                 self.height = lines.len();
///                 return old != self;
///             }
///         }
///         let mut width = child_layout.width;
///         let mut height = child_layout.width;
///         for attr in node.attributes() {
///             match attr.name {
///                 "width" => {
///                     width = attr.value.as_text().unwrap().parse().unwrap();
///                 }
///                 "height" => {
///                     height = attr.value.as_text().unwrap().parse().unwrap();
///                 }
///                 _ => unreachable!(),
///             }
///         }
///         self.width = width;
///         self.height = height;
///         old != self
///     }
/// }
/// ```
/// The generic argument (Depstate) must be a tuple containing any number of borrowed elments that are either a [ChildDepState], [ParentDepState] or [NodeDepState].
pub trait NodeDepState<DepState> {
    type Ctx;
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce(&mut self, node: NodeView, siblings: DepState, ctx: &Self::Ctx) -> bool;
}

/// Do not implement this trait. It is only meant to be derived and used through [crate::real_dom::RealDom].
pub trait State: Default + Clone {
    #[doc(hidden)]
    fn update<'a, T: Traversable<Node = Self, Id = ElementId>>(
        dirty: &[(ElementId, NodeMask)],
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

impl NodeDepState<()> for () {
    type Ctx = ();
    fn reduce(&mut self, _: NodeView, _sibling: (), _: &Self::Ctx) -> bool {
        false
    }
}

use crate::{ElementId, VNode};
use std::cell::Cell;

/// A list of VNodes with no single root.
pub struct VFragment<'src> {
    /// The key of the fragment to be used during keyed diffing.
    pub key: Option<&'src str>,

    /// The [`ElementId`] of the placeholder.
    pub placeholder: Cell<Option<ElementId>>,

    /// Fragments can never have zero children. Enforced by NodeFactory.
    ///
    /// You *can* make a fragment with no children, but it's not a valid fragment and your VDom will panic.
    pub children: &'src [VNode<'src>],
}

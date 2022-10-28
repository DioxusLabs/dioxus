use crate::ElementId;
use std::cell::Cell;

/// A bump-allocated string slice and metadata.
pub struct VText<'src> {
    /// The [`ElementId`] of the VText.
    pub id: Cell<Option<ElementId>>,

    /// The text of the VText.
    pub text: &'src str,
}

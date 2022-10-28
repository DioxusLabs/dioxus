use crate::ElementId;
use std::cell::Cell;

/// A placeholder node
pub struct VPlaceholder {
    pub id: Cell<Option<ElementId>>,
    pub dynamic_index: Option<usize>,
}

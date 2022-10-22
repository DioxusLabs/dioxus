use std::cell::Cell;

use crate::ElementId;

pub struct VPlaceholder {
    pub id: Cell<Option<ElementId>>,
    pub dynamic_index: Option<usize>,
}

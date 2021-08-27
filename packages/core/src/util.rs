use std::cell::Cell;

use crate::innerlude::*;

// create a cell with a "none" value
#[inline]
pub fn empty_cell() -> Cell<Option<ElementId>> {
    Cell::new(None)
}

pub fn type_name_of<T>(_: T) -> &'static str {
    std::any::type_name::<T>()
}

use std::cell::Cell;
use std::fmt::Display;

// create a cell with a "none" value
#[inline]
pub fn empty_cell() -> Cell<Option<ElementId>> {
    Cell::new(None)
}

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `ScopeId` will be reused for a new component.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ScopeId(pub usize);

/// An Element's unique identifier.
///
/// `ElementId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `ElementId` will be reused for a new component.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ElementId(pub usize);
impl Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ElementId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

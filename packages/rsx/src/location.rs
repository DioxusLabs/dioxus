use std::{cell::Cell, hash::Hash};

/// A simple idx in the code that can be used to track back to the original source location
///
/// Used in two places:
/// - In the `CallBody` to track the location of hotreloadable literals
/// - In the `Body` to track the ID of each template
///
/// We need an ID system, unfortunately, to properly disambiguate between different templates since
/// rustc assigns them all the same line!() and column!() information. Before, we hashed spans but
/// that has collision issues and is eventually relied on specifics of proc macros that aren't available
/// in testing (like snapshot testing). So, we just need an ID for each of these items, hence this struct.
///
/// This is "transparent" to partialeq and eq, so it will always return true when compared to another DynIdx.
#[derive(Clone, Debug, Default)]
pub struct DynIdx {
    idx: Cell<Option<usize>>,
}

impl PartialEq for DynIdx {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for DynIdx {}

impl DynIdx {
    pub fn set(&self, idx: usize) {
        self.idx.set(Some(idx));
    }

    pub fn get(&self) -> usize {
        self.idx.get().unwrap_or(usize::MAX)
    }
}

impl Hash for DynIdx {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.get().hash(state);
    }
}

use std::{cell::Cell, hash::Hash};

/// A simple idx in the code that can be used to track back to the original source location
///
/// Used in two places:
/// - In the `CallBody` to track the location of ifmt strings
/// - In the `Body` to track the ID of each template
///
/// We need an ID system, unfortunately, to properly disambiguate between different templates since
/// rustc assigns them all the same line!() and column!() information. Before, we hashed spans but
/// that has collision issues and is eventually relied on specifics of proc macros that aren't available
/// in testing (like snapshot testing). So, we just need an ID for each of these items, hence this struct.
#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct CallerLocation {
    idx: Cell<Option<usize>>,
}

impl CallerLocation {
    pub fn set(&self, idx: usize) {
        self.idx.set(Some(idx));
    }

    pub fn get(&self) -> usize {
        self.idx.get().unwrap_or(usize::MAX)
    }
}

impl Hash for CallerLocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.get().hash(state);
    }
}

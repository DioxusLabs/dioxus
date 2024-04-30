use std::{cell::Cell, hash::Hash};

/// Information about the location of the call to a component
///
/// This will be filled in when the dynamiccontext is built, filling in the file:line:column:id format
///
#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct CallerLocation {
    inner: Option<String>,
    pub idx: Cell<usize>,
    pub hotreload: Cell<bool>,
}

impl CallerLocation {
    pub fn set_idx(&self, idx: usize) {
        self.idx.set(idx);
    }
}

impl Hash for CallerLocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

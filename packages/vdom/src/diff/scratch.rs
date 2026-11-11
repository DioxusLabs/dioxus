//! Buffers kept on the [`VirtualDom`](crate::VirtualDom) and reused across diffs, so steady-state
//! renders don't allocate fresh working memory for every list they reconcile.

use super::iterator::{FragmentScratch, KeyedScratch};

#[derive(Default)]
pub(crate) struct DocumentScratch {
    /// Free fragment buffers. A fragment's buffers stay checked out while its children are
    /// diffed, so the pool grows to the deepest fragment nesting seen.
    fragments: Vec<FragmentScratch>,
    pub(super) keyed: KeyedScratch,
}

impl DocumentScratch {
    pub(super) fn take_fragment(&mut self) -> FragmentScratch {
        self.fragments.pop().unwrap_or_default()
    }

    pub(super) fn return_fragment(&mut self, scratch: FragmentScratch) {
        self.fragments.push(scratch);
    }
}

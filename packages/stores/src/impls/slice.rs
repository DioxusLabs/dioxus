//! `Vec<T>` read-side projector methods.

use std::iter::FusedIterator;
use std::ops::Index;

use crate::impls::index::IndexWrite;
use crate::{ProjectIndex, ProjectScope};
use dioxus_signals::Readable;

/// Read-side methods on `Vec<T>` projections.
pub trait ProjectSlice<T: 'static>: ProjectScope<Lens: Readable<Target = Vec<T>>> {
    /// Length; tracks shallowly.
    fn len(&self) -> usize {
        self.project_track_shallow();
        self.project_peek().len()
    }

    /// Is the slice empty? Tracks shallowly.
    fn is_empty(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_empty()
    }

    /// Iterate items, producing one indexed projection per element.
    fn iter(
        &self,
    ) -> impl ExactSizeIterator<
        Item = Self::Rebind<
            <<Self::Lens as Readable>::Target as Index<usize>>::Output,
            IndexWrite<usize, Self::Lens>,
        >,
    > + DoubleEndedIterator
           + FusedIterator
           + '_
    where
        Self: Clone + ProjectIndex<usize>,
        Self::Lens: 'static,
    {
        let len = ProjectSlice::len(self);
        let this = self.clone();
        (0..len).map(move |i| this.clone().index(i))
    }

    /// Try to get the item at `index` as a projection.
    fn get(
        self,
        index: usize,
    ) -> Option<
        Self::Rebind<
            <<Self::Lens as Readable>::Target as Index<usize>>::Output,
            IndexWrite<usize, Self::Lens>,
        >,
    >
    where
        Self: ProjectIndex<usize>,
        Self::Lens: 'static,
    {
        if index >= ProjectSlice::len(&self) {
            None
        } else {
            Some(self.index(index))
        }
    }
}

impl<T: 'static, P> ProjectSlice<T> for P where P: ProjectScope<Lens: Readable<Target = Vec<T>>> {}

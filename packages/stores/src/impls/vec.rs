//! `Vec<T>` mutation-side projector methods.

use crate::ProjectScope;
use dioxus_signals::{Writable, WritableExt};

/// Mutation methods on vector-shaped projections.
pub trait ProjectVec<T: 'static>: ProjectScope<Lens: Writable<Target = Vec<T>>> {
    /// Push an item to the end.
    fn push(&self, value: T) {
        self.project_mark_dirty_shallow();
        self.project_lens().write_unchecked().push(value);
    }

    /// Remove and return the item at `index`.
    fn remove(&self, index: usize) -> T {
        self.project_mark_dirty_shallow();
        self.project_mark_dirty_at_and_after_index(index);
        self.project_lens().write_unchecked().remove(index)
    }

    /// Insert an item at `index`.
    fn insert(&self, index: usize, value: T) {
        self.project_mark_dirty_shallow();
        self.project_mark_dirty_at_and_after_index(index);
        self.project_lens().write_unchecked().insert(index, value);
    }

    /// Clear all items.
    fn clear(&self) {
        self.project_mark_dirty();
        self.project_lens().write_unchecked().clear();
    }

    /// Retain only elements for which `f` returns true.
    fn retain(&self, mut f: impl FnMut(&T) -> bool) {
        let mut index = 0;
        let mut first_removed_index: Option<usize> = None;
        self.project_lens().write_unchecked().retain(|item| {
            let keep = f(item);
            if !keep {
                first_removed_index = first_removed_index.or(Some(index));
            }
            index += 1;
            keep
        });
        if let Some(index) = first_removed_index {
            self.project_mark_dirty_shallow();
            self.project_mark_dirty_at_and_after_index(index);
        }
    }
}

impl<T: 'static, P> ProjectVec<T> for P where P: ProjectScope<Lens: Writable<Target = Vec<T>>> {}

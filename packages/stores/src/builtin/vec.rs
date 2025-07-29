use crate::store::Store;
use dioxus_signals::Writable;

pub trait VecStoreExt {
    type Item;

    fn push(self, value: Self::Item);

    fn remove(self, index: usize) -> Self::Item;

    fn insert(self, index: usize, value: Self::Item);

    fn clear(self);

    fn retain(self, f: impl FnMut(&Self::Item) -> bool);
}

impl<W: Writable<Target = Vec<T>> + Copy + 'static, T: 'static> VecStoreExt for Store<Vec<T>, W> {
    type Item = T;

    fn push(self, value: Self::Item) {
        self.selector().mark_dirty_shallow();
        self.selector().write.write_unchecked().push(value);
    }

    fn remove(self, index: usize) -> Self::Item {
        self.selector().mark_dirty_shallow();
        self.selector().mark_dirty_at_and_after_index(index);
        self.selector().write.write_unchecked().remove(index)
    }

    fn insert(self, index: usize, value: Self::Item) {
        self.selector().mark_dirty_shallow();
        self.selector().mark_dirty_at_and_after_index(index);
        self.selector().write.write_unchecked().insert(index, value);
    }

    fn clear(self) {
        self.selector().mark_dirty();
        self.selector().write.write_unchecked().clear();
    }

    fn retain(self, mut f: impl FnMut(&Self::Item) -> bool) {
        let mut index = 0;
        let mut first_removed_index = None;
        self.selector().write.write_unchecked().retain(|item| {
            let keep = f(item);
            if !keep {
                first_removed_index = first_removed_index.or(Some(index));
            }
            index += 1;
            keep
        });
        if let Some(index) = first_removed_index {
            self.selector().mark_dirty_shallow();
            self.selector().mark_dirty_at_and_after_index(index);
        }
    }
}

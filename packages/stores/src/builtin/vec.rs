use crate::store::Store;
use dioxus_signals::Writable;

mod private {
    pub trait Sealed {}
}

/// A trait for `Store` that provides methods for working with `Vec` types.
///
/// # Example
/// ```rust, no_run
/// use dioxus_stores::*;
/// let mut store = use_store(|| vec![1, 2, 3]);
/// store.push(4);
/// ```
pub trait VecStoreExt: private::Sealed {
    /// The item type of the vector.
    type Item;

    /// Pushes an item to the end of the vector. This will only mark the length of the vector as dirty.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let mut store = use_store(|| vec![1, 2, 3]);
    /// store.push(4);
    /// ```
    fn push(&mut self, value: Self::Item);

    /// Removes an item from the vector at the specified index and returns it. This will mark items after
    /// the index and the length of the vector as dirty.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let mut store = use_store(|| vec![1, 2, 3]);
    /// let removed = store.remove(1);
    /// assert_eq!(removed, 2);
    /// ```
    fn remove(&mut self, index: usize) -> Self::Item;

    /// Inserts an item at the specified index in the vector. This will mark items at and after the index
    /// and the length of the vector as dirty.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let mut store = use_store(|| vec![1, 2, 3]);
    /// store.insert(1, 4);
    /// ```
    fn insert(&mut self, index: usize, value: Self::Item);

    /// Clears the vector, marking it as dirty.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let mut store = use_store(|| vec![1, 2, 3]);
    /// store.clear();
    /// ```
    fn clear(&mut self);

    /// Retains only the elements specified by the predicate. This will only mark the length of the vector
    /// and items after the first removed item as dirty.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let mut store = use_store(|| vec![1, 2, 3, 4, 5]);
    /// store.retain(|&x| x % 2 == 0);
    /// assert_eq!(store.len(), 2);
    /// ```
    fn retain(&mut self, f: impl FnMut(&Self::Item) -> bool);
}

impl<T, W> private::Sealed for Store<Vec<T>, W>
where
    W: Writable<Target = Vec<T>> + Copy + 'static,
    T: 'static,
{
}

impl<W: Writable<Target = Vec<T>> + Copy + 'static, T: 'static> VecStoreExt for Store<Vec<T>, W> {
    type Item = T;

    fn push(&mut self, value: Self::Item) {
        self.selector().mark_dirty_shallow();
        self.selector().write.write_unchecked().push(value);
    }

    fn remove(&mut self, index: usize) -> Self::Item {
        self.selector().mark_dirty_shallow();
        self.selector().mark_dirty_at_and_after_index(index);
        self.selector().write.write_unchecked().remove(index)
    }

    fn insert(&mut self, index: usize, value: Self::Item) {
        self.selector().mark_dirty_shallow();
        self.selector().mark_dirty_at_and_after_index(index);
        self.selector().write.write_unchecked().insert(index, value);
    }

    fn clear(&mut self) {
        self.selector().mark_dirty();
        self.selector().write.write_unchecked().clear();
    }

    fn retain(&mut self, mut f: impl FnMut(&Self::Item) -> bool) {
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

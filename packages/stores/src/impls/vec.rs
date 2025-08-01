use crate::store::Store;
use dioxus_signals::Writable;

impl<Lens: Writable<Target = Vec<T>> + 'static, T: 'static> Store<Vec<T>, Lens> {
    /// Pushes an item to the end of the vector. This will only mark the length of the vector as dirty.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let mut store = use_store(|| vec![1, 2, 3]);
    /// store.push(4);
    /// ```
    pub fn push(&mut self, value: T) {
        self.selector().mark_dirty_shallow();
        self.selector().write_untracked().push(value);
    }

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
    pub fn remove(&mut self, index: usize) -> T {
        self.selector().mark_dirty_shallow();
        self.selector().mark_dirty_at_and_after_index(index);
        self.selector().write_untracked().remove(index)
    }

    /// Inserts an item at the specified index in the vector. This will mark items at and after the index
    /// and the length of the vector as dirty.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let mut store = use_store(|| vec![1, 2, 3]);
    /// store.insert(1, 4);
    /// ```
    pub fn insert(&mut self, index: usize, value: T) {
        self.selector().mark_dirty_shallow();
        self.selector().mark_dirty_at_and_after_index(index);
        self.selector().write_untracked().insert(index, value);
    }

    /// Clears the vector, marking it as dirty.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let mut store = use_store(|| vec![1, 2, 3]);
    /// store.clear();
    /// ```
    pub fn clear(&mut self) {
        self.selector().mark_dirty();
        self.selector().write_untracked().clear();
    }

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
    pub fn retain(&mut self, mut f: impl FnMut(&T) -> bool) {
        let mut index = 0;
        let mut first_removed_index = None;
        self.selector().write_untracked().retain(|item| {
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

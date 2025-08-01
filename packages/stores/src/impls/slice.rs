use crate::store::Store;
use dioxus_signals::{MappedMutSignal, Readable, ReadableExt};

impl<W, I> Store<Vec<I>, W>
where
    W: Readable<Target = Vec<I>> + Copy + 'static,
    I: 'static,
{
    /// Returns the length of the slice. This will only track the shallow state of the slice.
    /// It will only cause a re-run if the length of the slice could change.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// assert_eq!(store.len(), 3);
    /// ```
    pub fn len(self) -> usize {
        self.selector().track_shallow();
        self.selector().peek().len()
    }

    /// Checks if the slice is empty. This will only track the shallow state of the slice.
    /// It will only cause a re-run if the length of the slice could change.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// assert!(!store.is_empty());
    /// ```
    pub fn is_empty(self) -> bool {
        self.selector().track_shallow();
        self.selector().peek().is_empty()
    }

    /// Returns an iterator over the items in the slice. This will only track the shallow state of the slice.
    /// It will only cause a re-run if the length of the slice could change.
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// for item in store.iter() {
    ///     println!("{}", item);
    /// }
    /// ```
    pub fn iter(
        self,
    ) -> impl Iterator<
        Item = Store<
            I,
            MappedMutSignal<
                I,
                W,
                impl Fn(&Vec<I>) -> &I + Copy + 'static,
                impl Fn(&mut Vec<I>) -> &mut I + Copy + 'static,
            >,
        >,
    > {
        (0..self.len()).map(move |i| self.index(i))
    }
}

use std::ops::DerefMut;
use std::ops::IndexMut;

use crate::store::Store;
use crate::IndexStoreExt;
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};

mod private {
    pub trait Sealed {}
}

/// A trait for `Store` that provides methods for working with slices.
///
/// # Example
/// ```rust, no_run
/// use dioxus_stores::*;
/// let store = use_store(|| vec![1, 2, 3]);
/// assert_eq!(store.len(), 3);
/// assert!(!store.is_empty());
/// for item in store.iter() {
///     println!("{}", item);
/// }
/// ```
pub trait SliceStoreExt: private::Sealed {
    /// The slice type of the store.
    type Slice;
    /// The item type of the slice.
    type Item;
    /// The writer backing the store.
    type Write;

    /// Returns the length of the slice. This will only track the shallow state of the slice.
    /// It will only cause a re-run if the length of the slice could change.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// assert_eq!(store.len(), 3);
    /// ```
    fn len(self) -> usize;

    /// Checks if the slice is empty. This will only track the shallow state of the slice.
    /// It will only cause a re-run if the length of the slice could change.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| vec![1, 2, 3]);
    /// assert!(!store.is_empty());
    /// ```
    fn is_empty(self) -> bool;

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
    fn iter(
        self,
    ) -> impl Iterator<
        Item = Store<
            Self::Item,
            MappedMutSignal<
                Self::Item,
                Self::Write,
                impl Fn(&Self::Slice) -> &Self::Item + Copy + 'static,
                impl Fn(&mut Self::Slice) -> &mut Self::Item + Copy + 'static,
            >,
        >,
    >;
}

impl<T: ?Sized, W> private::Sealed for Store<T, W> where W: Writable<Target = T> + Copy + 'static {}

impl<W, T, I> SliceStoreExt for Store<T, W>
where
    W: Writable<Target = T> + Copy + 'static,
    T: DerefMut<Target = [I]> + IndexMut<usize, Output = I> + 'static,
    I: 'static,
{
    type Slice = T;
    type Item = I;
    type Write = W;

    fn len(self) -> usize {
        self.selector().track_shallow();
        self.selector().write.read().deref().len()
    }

    fn is_empty(self) -> bool {
        self.selector().track_shallow();
        self.selector().write.read().deref().is_empty()
    }

    fn iter(
        self,
    ) -> impl Iterator<
        Item = Store<
            I,
            MappedMutSignal<
                I,
                W,
                impl Fn(&T) -> &I + Copy + 'static,
                impl Fn(&mut T) -> &mut I + Copy + 'static,
            >,
        >,
    > {
        (0..self.len()).map(move |i| IndexStoreExt::index(self, i))
    }
}

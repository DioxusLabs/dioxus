use crate::store::Store;
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};

mod private {
    pub trait Sealed {}
}

/// A trait for `Store` that provides methods for working with `Option` types.
///
/// # Example
/// ```rust, no_run
/// use dioxus_stores::*;
/// let store = use_store(|| Some(42));
/// assert!(store.is_some());
/// assert!(!store.is_none());
/// match store.transpose() {
///     Some(inner_store) => assert_eq!(inner_store(), 42),
///     None => panic!("Expected Some"),
/// }
/// ```
pub trait OptionStoreExt: private::Sealed {
    /// The data type contained in the `Option`.
    type Data;
    /// The writer backing the store.
    type Write;

    /// Checks if the `Option` is `Some`. This will only track the shallow state of the `Option`. It will
    /// only cause a re-run if the `Option` could change from `None` to `Some` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42));
    /// assert!(store.is_some());
    /// ```
    fn is_some(self) -> bool;

    /// Checks if the `Option` is `None`. This will only track the shallow state of the `Option`. It will
    /// only cause a re-run if the `Option` could change from `Some` to `None` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| None::<i32>);
    /// assert!(store.is_none());
    /// ```
    fn is_none(self) -> bool;

    /// Transpose the `Store<Option<T>>` into a `Option<Store<T>>`. This will only track the shallow state of the `Option`. It will
    /// only cause a re-run if the `Option` could change from `None` to `Some` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42));
    /// let transposed = store.transpose();
    /// match transposed {
    ///     Some(inner_store) => assert_eq!(inner_store(), 42),
    ///     None => panic!("Expected Some"),
    /// }
    /// ```
    fn transpose(
        self,
    ) -> Option<
        Store<
            Self::Data,
            MappedMutSignal<
                Self::Data,
                Self::Write,
                impl Fn(&Option<Self::Data>) -> &Self::Data + Copy + 'static,
                impl Fn(&mut Option<Self::Data>) -> &mut Self::Data + Copy + 'static,
            >,
        >,
    >;

    /// Unwraps the `Option` and returns a `Store<T>`. This will only track the shallow state of the `Option`. It will
    /// only cause a re-run if the `Option` could change from `None` to `Some` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42));
    /// let unwrapped = store.unwrap();
    /// assert_eq!(unwrapped(), 42);
    /// ```
    fn unwrap(
        self,
    ) -> Store<
        Self::Data,
        MappedMutSignal<
            Self::Data,
            Self::Write,
            impl Fn(&Option<Self::Data>) -> &Self::Data + Copy + 'static,
            impl Fn(&mut Option<Self::Data>) -> &mut Self::Data + Copy + 'static,
        >,
    >;
}

impl<W: Writable<Target = Option<T>> + Copy + 'static, T: 'static> private::Sealed
    for Store<Option<T>, W>
{
}

impl<W: Writable<Target = Option<T>> + Copy + 'static, T: 'static> OptionStoreExt
    for Store<Option<T>, W>
{
    type Data = T;
    type Write = W;

    fn is_some(self) -> bool {
        self.selector().track_shallow();
        self.selector().write.read().is_some()
    }

    fn is_none(self) -> bool {
        self.selector().track_shallow();
        self.selector().write.read().is_none()
    }

    fn transpose(
        self,
    ) -> Option<
        Store<
            T,
            MappedMutSignal<
                T,
                W,
                impl Fn(&Option<T>) -> &T + Copy + 'static,
                impl Fn(&mut Option<T>) -> &mut T + Copy + 'static,
            >,
        >,
    > {
        self.is_some().then(|| {
            self.selector()
                .child(
                    0,
                    move |value: &Option<T>| {
                        value.as_ref().unwrap_or_else(|| {
                            panic!("Tried to access `Some` on an Option value");
                        })
                    },
                    move |value: &mut Option<T>| {
                        value.as_mut().unwrap_or_else(|| {
                            panic!("Tried to access `Some` on an Option value");
                        })
                    },
                )
                .into()
        })
    }

    fn unwrap(
        self,
    ) -> Store<
        T,
        MappedMutSignal<
            T,
            W,
            impl Fn(&Option<T>) -> &T + Copy + 'static,
            impl Fn(&mut Option<T>) -> &mut T + Copy + 'static,
        >,
    > {
        self.transpose().unwrap()
    }
}

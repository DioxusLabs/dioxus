use crate::store::Store;
use dioxus_signals::{MappedMutSignal, Readable, ReadableExt};

impl<W: Readable<Target = Option<T>> + 'static, T: 'static> Store<Option<T>, W> {
    /// Checks if the `Option` is `Some`. This will only track the shallow state of the `Option`. It will
    /// only cause a re-run if the `Option` could change from `None` to `Some` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42));
    /// assert!(store.is_some());
    /// ```
    pub fn is_some(&self) -> bool {
        self.selector().track_shallow();
        self.selector().peek().is_some()
    }

    /// Checks if the `Option` is `None`. This will only track the shallow state of the `Option`. It will
    /// only cause a re-run if the `Option` could change from `Some` to `None` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| None::<i32>);
    /// assert!(store.is_none());
    /// ```
    pub fn is_none(&self) -> bool {
        self.selector().track_shallow();
        self.selector().peek().is_none()
    }

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
    pub fn transpose(self) -> Option<Store<T, MappedMutSignal<T, W>>> {
        self.is_some().then(move || {
            let map: fn(&Option<T>) -> &T = |value| {
                value.as_ref().unwrap_or_else(|| {
                    panic!("Tried to access `Some` on an Option value");
                })
            };
            let map_mut: fn(&mut Option<T>) -> &mut T = |value| {
                value.as_mut().unwrap_or_else(|| {
                    panic!("Tried to access `Some` on an Option value");
                })
            };
            self.into_selector().child(0, map, map_mut).into()
        })
    }

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
    pub fn unwrap(self) -> Store<T, MappedMutSignal<T, W>> {
        self.transpose().unwrap()
    }
}

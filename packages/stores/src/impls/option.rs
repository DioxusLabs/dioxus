use std::ops::DerefMut;

use crate::{store::Store, MappedStore};
use dioxus_signals::{Readable, ReadableExt};

impl<Lens: Readable<Target = Option<T>> + 'static, T: 'static> Store<Option<T>, Lens> {
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

    /// Returns true if the option is Some and the closure returns true. This will always track the shallow
    /// state of the and will track the inner state of the enum if the enum is Some.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42));
    /// assert!(store.is_some_and(|v| *v == 42));
    /// ```
    pub fn is_some_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.selector().track_shallow();
        let value = self.selector().peek();
        if let Some(v) = &*value {
            self.selector().track();
            f(v)
        } else {
            false
        }
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

    /// Returns true if the option is None or the closure returns true. This will always track the shallow
    /// state of the and will track the inner state of the enum if the enum is Some.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42));
    /// assert!(store.is_none_or(|v| *v == 42));
    /// ```
    pub fn is_none_or(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.selector().track_shallow();
        let value = self.selector().peek();
        if let Some(v) = &*value {
            self.selector().track();
            f(v)
        } else {
            true
        }
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
    pub fn transpose(self) -> Option<MappedStore<T, Lens>> {
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
    pub fn unwrap(self) -> MappedStore<T, Lens> {
        self.transpose().unwrap()
    }

    /// Expects the `Option` to be `Some` and returns a `Store<T>`. If the value is `None`, this will panic with `msg`. This will
    /// only track the shallow state of the `Option`. It will only cause a re-run if the `Option` could change from `None`
    /// to `Some` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42));
    /// let unwrapped = store.expect("the answer to life the universe and everything");
    /// assert_eq!(unwrapped(), 42);
    /// ```
    pub fn expect(self, msg: &str) -> MappedStore<T, Lens> {
        self.transpose().expect(msg)
    }

    /// Returns a slice of the contained value, or an empty slice. This will not subscribe to any part of the store.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus::prelude::*;
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42));
    /// let slice = store.as_slice();
    /// assert_eq!(&*slice.read(), [42]);
    /// ```
    pub fn as_slice(self) -> MappedStore<[T], Lens> {
        let map: fn(&Option<T>) -> &[T] = |value| value.as_slice();
        let map_mut: fn(&mut Option<T>) -> &mut [T] = |value| value.as_mut_slice();
        self.into_selector().map(map, map_mut).into()
    }

    /// Transpose the store then coerce the contents of the Option with deref. This will only track the shallow state of the `Option`. It will
    /// only cause a re-run if the `Option` could change from `None` to `Some` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(Box::new(42)));
    /// let derefed = store.as_deref().unwrap();
    /// assert_eq!(derefed(), 42);
    /// ```
    pub fn as_deref(self) -> Option<MappedStore<T::Target, Lens>>
    where
        T: DerefMut,
    {
        self.is_some().then(move || {
            let map: fn(&Option<T>) -> &T::Target = |value| {
                value
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!("Tried to access `Some` on an Option value");
                    })
                    .deref()
            };
            let map_mut: fn(&mut Option<T>) -> &mut T::Target = |value| {
                value
                    .as_mut()
                    .unwrap_or_else(|| {
                        panic!("Tried to access `Some` on an Option value");
                    })
                    .deref_mut()
            };
            self.into_selector().child(0, map, map_mut).into()
        })
    }

    /// Transpose the store then filter the contents of the Option with a closure. This will always track the shallow
    /// state of the and will track the inner state of the enum if the enum is Some.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42));
    /// let option = store.filter(|&v| v > 40);
    /// let value = option.unwrap();
    /// assert_eq!(value(), 42);
    /// ```
    pub fn filter(self, f: impl FnOnce(&T) -> bool) -> Option<MappedStore<T, Lens>> {
        self.is_some_and(f).then(move || {
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

    /// Call the function with a reference to the inner value if it is Some. This will always track the shallow
    /// state of the and will track the inner state of the enum if the enum is Some.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Some(42)).inspect(|v| println!("{v}"));
    /// ```
    pub fn inspect(self, f: impl FnOnce(&T)) -> Self {
        {
            self.selector().track_shallow();
            let value = self.selector().peek();
            if let Some(v) = &*value {
                self.selector().track();
                f(v);
            }
        }
        self
    }
}

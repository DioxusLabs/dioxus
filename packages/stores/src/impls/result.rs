use std::fmt::Debug;

use crate::{store::Store, MappedStore};
use dioxus_signals::{Readable, ReadableExt, Writable};

impl<Lens, T, E> Store<Result<T, E>, Lens>
where
    Lens: Readable<Target = Result<T, E>> + 'static,
    T: 'static,
    E: 'static,
{
    /// Checks if the `Result` is `Ok`. This will only track the shallow state of the `Result`. It will
    /// only cause a re-run if the `Result` could change from `Err` to `Ok` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Ok::<u32, ()>(42));
    /// assert!(store.is_ok());
    /// ```
    pub fn is_ok(&self) -> bool {
        self.selector().track_shallow();
        self.selector().peek().is_ok()
    }

    /// Returns true if the result is Ok and the closure returns true. This will always track the shallow
    /// state of the and will track the inner state of the enum if the enum is Ok.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Ok::<u32, ()>(42));
    /// assert!(store.is_ok_and(|v| *v == 42));
    /// ```
    pub fn is_ok_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.selector().track_shallow();
        let value = self.selector().peek();
        if let Ok(v) = &*value {
            self.selector().track();
            f(v)
        } else {
            false
        }
    }

    /// Checks if the `Result` is `Err`. This will only track the shallow state of the `Result`. It will
    /// only cause a re-run if the `Result` could change from `Ok` to `Err` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Err::<(), u32>(42));
    /// assert!(store.is_err());
    /// ```
    pub fn is_err(&self) -> bool {
        self.selector().track_shallow();
        self.selector().peek().is_err()
    }

    /// Returns true if the result is Err and the closure returns true. This will always track the shallow
    /// state of the and will track the inner state of the enum if the enum is Err.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Err::<(), u32>(42));
    /// assert!(store.is_err_and(|v| *v == 42));
    /// ```
    pub fn is_err_and(&self, f: impl FnOnce(&E) -> bool) -> bool {
        self.selector().track_shallow();
        let value = self.selector().peek();
        if let Err(e) = &*value {
            self.selector().track();
            f(e)
        } else {
            false
        }
    }

    /// Converts `Store<Result<T, E>>` into `Option<Store<T>>`, discarding the error if present. This will
    /// only track the shallow state of the `Result`. It will only cause a re-run if the `Result` could
    /// change from `Err` to `Ok` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Ok::<u32, ()>(42));
    /// match store.ok() {
    ///     Some(ok_store) => assert_eq!(ok_store(), 42),
    ///     None => panic!("Expected Ok"),
    /// }
    /// ```
    pub fn ok(self) -> Option<MappedStore<T, Lens>> {
        let map: fn(&Result<T, E>) -> &T = |value| {
            value.as_ref().unwrap_or_else(|_| {
                panic!("Tried to access `ok` on an Err value");
            })
        };
        let map_mut: fn(&mut Result<T, E>) -> &mut T = |value| {
            value.as_mut().unwrap_or_else(|_| {
                panic!("Tried to access `ok` on an Err value");
            })
        };
        self.is_ok()
            .then(|| self.into_selector().child(0, map, map_mut).into())
    }

    /// Converts `Store<Result<T, E>>` into `Option<Store<E>>`, discarding the success if present. This will
    /// only track the shallow state of the `Result`. It will only cause a re-run if the `Result` could
    /// change from `Ok` to `Err` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Err::<(), u32>(42));
    /// match store.err() {
    ///     Some(err_store) => assert_eq!(err_store(), 42),
    ///     None => panic!("Expected Err"),
    /// }
    /// ```
    pub fn err(self) -> Option<MappedStore<E, Lens>>
    where
        Lens: Writable<Target = Result<T, E>> + 'static,
    {
        self.is_err().then(|| {
            let map: fn(&Result<T, E>) -> &E = |value| match value {
                Ok(_) => panic!("Tried to access `err` on an Ok value"),
                Err(e) => e,
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |value| match value {
                Ok(_) => panic!("Tried to access `err` on an Ok value"),
                Err(e) => e,
            };
            self.into_selector().child(1, map, map_mut).into()
        })
    }

    /// Transposes the `Store<Result<T, E>>` into a `Result<Store<T>, Store<E>>`. This will only track the
    /// shallow state of the `Result`. It will only cause a re-run if the `Result` could change from `Err` to
    /// `Ok` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Ok::<u32, ()>(42));
    /// match store.transpose() {
    ///     Ok(ok_store) => assert_eq!(ok_store(), 42),
    ///     Err(err_store) => assert_eq!(err_store(), ()),
    /// }
    /// ```
    #[allow(clippy::result_large_err)]
    pub fn transpose(self) -> Result<MappedStore<T, Lens>, MappedStore<E, Lens>>
    where
        Lens: Writable<Target = Result<T, E>> + 'static,
    {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T = |value| match value {
                Ok(t) => t,
                Err(_) => panic!("Tried to access `ok` on an Err value"),
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut T = |value| match value {
                Ok(t) => t,
                Err(_) => panic!("Tried to access `ok` on an Err value"),
            };
            Ok(self.into_selector().child(0, map, map_mut).into())
        } else {
            let map: fn(&Result<T, E>) -> &E = |value| match value {
                Ok(_) => panic!("Tried to access `err` on an Ok value"),
                Err(e) => e,
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |value| match value {
                Ok(_) => panic!("Tried to access `err` on an Ok value"),
                Err(e) => e,
            };
            Err(self.into_selector().child(1, map, map_mut).into())
        }
    }

    /// Unwraps the `Result` and returns a `Store<T>`. This will only track the shallow state of the `Result`.
    /// It will only cause a re-run if the `Result` could change from `Err` to `Ok` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Ok::<u32, ()>(42));
    /// let unwrapped = store.unwrap();
    /// assert_eq!(unwrapped(), 42);
    /// ```
    pub fn unwrap(self) -> MappedStore<T, Lens>
    where
        Lens: Writable<Target = Result<T, E>> + 'static,
        E: Debug,
    {
        self.transpose().unwrap()
    }

    /// Expects the `Result` to be `Ok` and returns a `Store<T>`. If the value is `Err`, this will panic with `msg`.
    /// This will only track the shallow state of the `Result`. It will only cause a re-run if the `Result` could
    /// change from `Err` to `Ok` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Ok::<u32, ()>(42));
    /// let unwrapped = store.expect("Expected Ok");
    /// assert_eq!(unwrapped(), 42);
    /// ```
    pub fn expect(self, msg: &str) -> MappedStore<T, Lens>
    where
        Lens: Writable<Target = Result<T, E>> + 'static,
        E: Debug,
    {
        self.transpose().expect(msg)
    }

    /// Unwraps the error variant of the `Result`. This will only track the shallow state of the `Result`.
    /// It will only cause a re-run if the `Result` could change from `Ok` to `Err` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Err::<(), u32>(42));
    /// let unwrapped_err = store.unwrap_err();
    /// assert_eq!(unwrapped_err(), 42);
    /// ```
    pub fn unwrap_err(self) -> MappedStore<E, Lens>
    where
        Lens: Writable<Target = Result<T, E>> + 'static,
        T: Debug,
    {
        self.transpose().unwrap_err()
    }

    /// Expects the `Result` to be `Err` and returns a `Store<E>`. If the value is `Ok`, this will panic with `msg`.
    /// This will only track the shallow state of the `Result`. It will only cause a re-run if the `Result` could
    /// change from `Ok` to `Err` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Err::<(), u32>(42));
    /// let unwrapped_err = store.expect_err("Expected Err");
    /// assert_eq!(unwrapped_err(), 42);
    /// ```
    pub fn expect_err(self, msg: &str) -> MappedStore<E, Lens>
    where
        Lens: Writable<Target = Result<T, E>> + 'static,
        T: Debug,
    {
        self.transpose().expect_err(msg)
    }

    /// Call the function with a reference to the inner value if it is Ok. This will always track the shallow
    /// state of the and will track the inner state of the enum if the enum is Ok.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Ok::<u32, ()>(42)).inspect(|v| println!("{v}"));
    /// ```
    pub fn inspect(self, f: impl FnOnce(&T)) -> Self
    where
        Lens: Writable<Target = Result<T, E>> + 'static,
    {
        {
            self.selector().track_shallow();
            let value = self.selector().peek();
            if let Ok(value) = &*value {
                self.selector().track();
                f(value);
            }
        }
        self
    }

    /// Call the function with a mutable reference to the inner value if it is Err. This will always track the shallow
    /// state of the `Result` and will track the inner state of the enum if the enum is Err.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Err::<(), u32>(42)).inspect_err(|v| println!("{v}"));
    /// ```
    pub fn inspect_err(self, f: impl FnOnce(&E)) -> Self
    where
        Lens: Writable<Target = Result<T, E>> + 'static,
    {
        {
            self.selector().track_shallow();
            let value = self.selector().peek();
            if let Err(value) = &*value {
                self.selector().track();
                f(value);
            }
        }
        self
    }

    /// Transpose the store then coerce the contents of the Result with deref. This will only track the shallow state of the `Result`. It will
    /// only cause a re-run if the `Result` could change from `Err` to `Ok` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Ok::<Box<u32>, ()>(Box::new(42)));
    /// let derefed = store.as_deref().unwrap();
    /// assert_eq!(derefed(), 42);
    /// ```
    pub fn as_deref(self) -> Result<MappedStore<T::Target, Lens>, MappedStore<E, Lens>>
    where
        Lens: Writable<Target = Result<T, E>> + 'static,
        T: std::ops::DerefMut,
    {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T::Target = |value| match value {
                Ok(t) => t.deref(),
                Err(_) => panic!("Tried to access `ok` on an Err value"),
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut T::Target = |value| match value {
                Ok(t) => t.deref_mut(),
                Err(_) => panic!("Tried to access `ok` on an Err value"),
            };
            Ok(self.into_selector().child(0, map, map_mut).into())
        } else {
            let map: fn(&Result<T, E>) -> &E = |value| match value {
                Ok(_) => panic!("Tried to access `err` on an Ok value"),
                Err(e) => e,
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |value| match value {
                Ok(_) => panic!("Tried to access `err` on an Ok value"),
                Err(e) => e,
            };
            Err(self.into_selector().child(1, map, map_mut).into())
        }
    }
}

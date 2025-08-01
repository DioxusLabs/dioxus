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
}

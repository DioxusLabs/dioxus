use crate::store::Store;
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};

impl<W, T, E> Store<Result<T, E>, W>
where
    W: Writable<Target = Result<T, E>> + Copy + 'static,
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
    pub fn is_ok(self) -> bool {
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
    pub fn is_err(self) -> bool {
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
    pub fn ok(
        self,
    ) -> Option<
        Store<
            T,
            MappedMutSignal<
                T,
                W,
                impl Fn(&Result<T, E>) -> &T + Copy + 'static,
                impl Fn(&mut Result<T, E>) -> &mut T + Copy + 'static,
            >,
        >,
    > {
        self.is_ok().then(|| {
            self.selector()
                .child(
                    0,
                    move |value: &Result<T, E>| {
                        value.as_ref().unwrap_or_else(|_| {
                            panic!("Tried to access `ok` on an Err value");
                        })
                    },
                    move |value: &mut Result<T, E>| {
                        value.as_mut().unwrap_or_else(|_| {
                            panic!("Tried to access `ok` on an Err value");
                        })
                    },
                )
                .into()
        })
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
    pub fn err(
        self,
    ) -> Option<
        Store<
            E,
            MappedMutSignal<
                E,
                W,
                impl Fn(&Result<T, E>) -> &E + Copy + 'static,
                impl Fn(&mut Result<T, E>) -> &mut E + Copy + 'static,
            >,
        >,
    >
    where
        W: Writable<Target = Result<T, E>> + Copy + 'static,
    {
        self.is_err().then(|| {
            self.selector()
                .child(
                    1,
                    move |value: &Result<T, E>| match value {
                        Ok(_) => panic!("Tried to access `err` on an Ok value"),
                        Err(e) => e,
                    },
                    move |value: &mut Result<T, E>| match value {
                        Ok(_) => panic!("Tried to access `err` on an Ok value"),
                        Err(e) => e,
                    },
                )
                .into()
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
    pub fn transpose(
        self,
    ) -> Result<
        Store<
            T,
            MappedMutSignal<
                T,
                W,
                impl Fn(&Result<T, E>) -> &T + Copy + 'static,
                impl Fn(&mut Result<T, E>) -> &mut T + Copy + 'static,
            >,
        >,
        Store<
            E,
            MappedMutSignal<
                E,
                W,
                impl Fn(&Result<T, E>) -> &E + Copy + 'static,
                impl Fn(&mut Result<T, E>) -> &mut E + Copy + 'static,
            >,
        >,
    >
    where
        W: Writable<Target = Result<T, E>> + Copy + 'static,
    {
        if self.is_ok() {
            Ok(self
                .selector()
                .child(
                    0,
                    move |value: &Result<T, E>| {
                        value.as_ref().unwrap_or_else(|_| {
                            panic!("Tried to access `ok` on an Err value");
                        })
                    },
                    move |value: &mut Result<T, E>| {
                        value.as_mut().unwrap_or_else(|_| {
                            panic!("Tried to access `ok` on an Err value");
                        })
                    },
                )
                .into())
        } else {
            Err(self
                .selector()
                .child(
                    1,
                    move |value: &Result<T, E>| match value {
                        Ok(_) => panic!("Tried to access `err` on an Ok value"),
                        Err(e) => e,
                    },
                    move |value: &mut Result<T, E>| match value {
                        Ok(_) => panic!("Tried to access `err` on an Ok value"),
                        Err(e) => e,
                    },
                )
                .into())
        }
    }
}

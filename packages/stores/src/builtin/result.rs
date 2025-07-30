use crate::store::Store;
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};

mod private {
    pub trait Sealed {}
}

/// A trait for `Store` that provides methods for working with `Result` types.
///
/// # Example
/// ```rust, no_run
/// use dioxus_stores::*;
/// let store = use_store(|| Ok::<i32, ()>(42));
/// assert!(store.is_ok());
/// assert!(!store.is_err());
/// match store.transpose() {
///     Ok(inner_store) => assert_eq!(inner_store(), 42),
///     Err(_) => panic!("Expected Ok"),
/// }
/// ```
pub trait ResultStoreExt: private::Sealed {
    /// The type of the `Ok` variant.
    type Ok;
    /// The type of the `Err` variant.
    type Err;
    /// The writer backing the store.
    type Write;

    /// Checks if the `Result` is `Ok`. This will only track the shallow state of the `Result`. It will
    /// only cause a re-run if the `Result` could change from `Err` to `Ok` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Ok::<u32, ()>(42));
    /// assert!(store.is_ok());
    /// ```
    fn is_ok(self) -> bool;

    /// Checks if the `Result` is `Err`. This will only track the shallow state of the `Result`. It will
    /// only cause a re-run if the `Result` could change from `Ok` to `Err` or vice versa.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus_stores::*;
    /// let store = use_store(|| Err::<(), u32>(42));
    /// assert!(store.is_err());
    /// ```
    fn is_err(self) -> bool;

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
    fn ok(
        self,
    ) -> Option<
        Store<
            Self::Ok,
            MappedMutSignal<
                Self::Ok,
                Self::Write,
                impl Fn(&Result<Self::Ok, Self::Err>) -> &Self::Ok + Copy + 'static,
                impl Fn(&mut Result<Self::Ok, Self::Err>) -> &mut Self::Ok + Copy + 'static,
            >,
        >,
    >;

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
    fn err(
        self,
    ) -> Option<
        Store<
            Self::Err,
            MappedMutSignal<
                Self::Err,
                Self::Write,
                impl Fn(&Result<Self::Ok, Self::Err>) -> &Self::Err + Copy + 'static,
                impl Fn(&mut Result<Self::Ok, Self::Err>) -> &mut Self::Err + Copy + 'static,
            >,
        >,
    >;

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
    fn transpose(
        self,
    ) -> Result<
        Store<
            Self::Ok,
            MappedMutSignal<
                Self::Ok,
                Self::Write,
                impl Fn(&Result<Self::Ok, Self::Err>) -> &Self::Ok + Copy + 'static,
                impl Fn(&mut Result<Self::Ok, Self::Err>) -> &mut Self::Ok + Copy + 'static,
            >,
        >,
        Store<
            Self::Err,
            MappedMutSignal<
                Self::Err,
                Self::Write,
                impl Fn(&Result<Self::Ok, Self::Err>) -> &Self::Err + Copy + 'static,
                impl Fn(&mut Result<Self::Ok, Self::Err>) -> &mut Self::Err + Copy + 'static,
            >,
        >,
    >;
}

impl<W, T, E> private::Sealed for Store<Result<T, E>, W>
where
    W: Writable<Target = Result<T, E>> + Copy + 'static,
    T: 'static,
    E: 'static,
{
}

impl<W, T, E> ResultStoreExt for Store<Result<T, E>, W>
where
    W: Writable<Target = Result<T, E>> + Copy + 'static,
    T: 'static,
    E: 'static,
{
    type Ok = T;
    type Err = E;
    type Write = W;

    fn is_ok(self) -> bool {
        self.selector().track_shallow();
        self.selector().write.read().is_ok()
    }

    fn is_err(self) -> bool {
        self.selector().track_shallow();
        self.selector().write.read().is_err()
    }

    fn ok(
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

    fn err(
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

    fn transpose(
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

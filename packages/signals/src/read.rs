use std::ops::Index;

use dioxus_core::Subscribers;
use generational_box::{AnyStorage, UnsyncStorage};

use crate::{MappedSignal, ReadSignal};

/// A reference to a value that can be read from.
#[allow(type_alias_bounds)]
pub type ReadableRef<'a, T: Readable, O = <T as Readable>::Target> =
    <T::Storage as AnyStorage>::Ref<'a, O>;

/// A trait for states that can be read from like [`crate::Signal`], [`crate::GlobalSignal`], or [`crate::ReadSignal`]. You may choose to accept this trait as a parameter instead of the concrete type to allow for more flexibility in your API. For example, instead of creating two functions, one that accepts a [`crate::Signal`] and one that accepts a [`crate::GlobalSignal`], you can create one function that accepts a [`Readable`] type.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// fn double(something_readable: &impl Readable<Target = i32>) -> i32 {
///     something_readable.cloned() * 2
/// }
///
/// static COUNT: GlobalSignal<i32> = Signal::global(|| 0);
///
/// fn MyComponent(count: Signal<i32>) -> Element {
///     // Since we defined the function in terms of the readable trait, we can use it with any readable type (Signal, GlobalSignal, ReadSignal, etc)
///     let doubled = use_memo(move || double(&count));
///     let global_count_doubled = use_memo(|| double(&COUNT));
///     rsx! {
///         div {
///             "Count local: {count}"
///             "Doubled local: {doubled}"
///             "Count global: {COUNT}"
///             "Doubled global: {global_count_doubled}"
///         }
///     }
/// }
/// ```
pub trait Readable {
    /// The target type of the reference.
    type Target: ?Sized;

    /// The type of the storage this readable uses.
    type Storage: AnyStorage;

    /// Try to get a reference to the value without checking the lifetime. This will subscribe the current scope to the signal.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        Self::Target: 'static;

    /// Try to peek the current value of the signal without subscribing to updates. If the value has
    /// been dropped, this will return an error.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    fn try_peek_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        Self::Target: 'static;

    /// Get the underlying subscriber list for this readable. This is used to track when the value changes and notify subscribers.
    fn subscribers(&self) -> Subscribers
    where
        Self::Target: 'static;
}

/// An extension trait for `Readable` types that provides some convenience methods.
pub trait ReadableExt: Readable {
    /// Get the current value of the state. If this is a signal, this will subscribe the current scope to the signal.
    /// If the value has been dropped, this will panic. Calling this on a Signal is the same as
    /// using the signal() syntax to read and subscribe to its value
    #[track_caller]
    fn read(&self) -> ReadableRef<'_, Self>
    where
        Self::Target: 'static,
    {
        self.try_read().unwrap()
    }

    /// Try to get the current value of the state. If this is a signal, this will subscribe the current scope to the signal.
    #[track_caller]
    fn try_read(&self) -> Result<ReadableRef<'_, Self>, generational_box::BorrowError>
    where
        Self::Target: 'static,
    {
        self.try_read_unchecked()
            .map(Self::Storage::downcast_lifetime_ref)
    }

    /// Get a reference to the value without checking the lifetime. This will subscribe the current scope to the signal.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    #[track_caller]
    fn read_unchecked(&self) -> ReadableRef<'static, Self>
    where
        Self::Target: 'static,
    {
        self.try_read_unchecked().unwrap()
    }

    /// Get the current value of the state without subscribing to updates. If the value has been dropped, this will panic.
    ///
    /// # Example
    /// ```rust
    /// # use dioxus::prelude::*;
    /// fn MyComponent(mut count: Signal<i32>) -> Element {
    ///     let mut event_source = use_signal(|| None);
    ///     let doubled = use_memo(move || {
    ///         // We want to log the value of the event_source, but we don't need to rerun the doubled value if the event_source changes (because the value of doubled doesn't depend on the event_source)
    ///         // We can read the value with peek without subscribing to updates
    ///         let source = event_source.peek();
    ///         tracing::info!("Clicked: {source:?}");
    ///         count() * 2
    ///     });
    ///     rsx! {
    ///         div { "Count: {count}" }
    ///         div { "Doubled: {doubled}" }
    ///         button {
    ///             onclick: move |_| {
    ///                 event_source.set(Some("Click me button"));
    ///                 count += 1;
    ///             },
    ///             "Click me"
    ///         }
    ///         button {
    ///             onclick: move |_| {
    ///                 event_source.set(Some("Double me button"));
    ///                 count += 1;
    ///             },
    ///             "Double me"
    ///         }
    ///     }
    /// }
    /// ```
    #[track_caller]
    fn peek(&self) -> ReadableRef<'_, Self>
    where
        Self::Target: 'static,
    {
        Self::Storage::downcast_lifetime_ref(self.peek_unchecked())
    }

    /// Try to peek the current value of the signal without subscribing to updates. If the value has
    /// been dropped, this will return an error.
    #[track_caller]
    fn try_peek(&self) -> Result<ReadableRef<'_, Self>, generational_box::BorrowError>
    where
        Self::Target: 'static,
    {
        self.try_peek_unchecked()
            .map(Self::Storage::downcast_lifetime_ref)
    }

    /// Get the current value of the signal without checking the lifetime. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    fn peek_unchecked(&self) -> ReadableRef<'static, Self>
    where
        Self::Target: 'static,
    {
        self.try_peek_unchecked().unwrap()
    }

    /// Map the references of the readable value to a new type. This lets you provide a view
    /// into the readable value without creating a new signal or cloning the value.
    ///
    /// Anything that subscribes to the readable value will be rerun whenever the original value changes, even if the view does not
    /// change. If you want to memorize the view, you can use a [`crate::Memo`] instead. For fine grained scoped updates, use
    /// stores instead
    ///
    /// # Example
    /// ```rust
    /// # use dioxus::prelude::*;
    /// fn List(list: Signal<Vec<i32>>) -> Element {
    ///     rsx! {
    ///         for index in 0..list.len() {
    ///             // We can use the `map` method to provide a view into the single item in the list that the child component will render
    ///             Item { item: list.map(move |v| &v[index]) }
    ///         }
    ///     }
    /// }
    ///
    /// // The child component doesn't need to know that the mapped value is coming from a list
    /// #[component]
    /// fn Item(item: ReadSignal<i32>) -> Element {
    ///     rsx! {
    ///         div { "Item: {item}" }
    ///     }
    /// }
    /// ```
    fn map<F, O>(self, f: F) -> MappedSignal<O, Self, F>
    where
        Self: Clone + Sized,
        F: Fn(&Self::Target) -> &O,
    {
        MappedSignal::new(self, f)
    }

    /// Clone the inner value and return it. If the value has been dropped, this will panic.
    #[track_caller]
    fn cloned(&self) -> Self::Target
    where
        Self::Target: Clone + 'static,
    {
        self.read().clone()
    }

    /// Run a function with a reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn with<O>(&self, f: impl FnOnce(&Self::Target) -> O) -> O
    where
        Self::Target: 'static,
    {
        f(&*self.read())
    }

    /// Run a function with a reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn with_peek<O>(&self, f: impl FnOnce(&Self::Target) -> O) -> O
    where
        Self::Target: 'static,
    {
        f(&*self.peek())
    }

    /// Index into the inner value and return a reference to the result. If the value has been dropped or the index is invalid, this will panic.
    #[track_caller]
    fn index<I>(
        &self,
        index: I,
    ) -> ReadableRef<'_, Self, <Self::Target as std::ops::Index<I>>::Output>
    where
        Self::Target: std::ops::Index<I> + 'static,
    {
        <Self::Storage as AnyStorage>::map(self.read(), |v| v.index(index))
    }

    /// Casts this [`Readable`] into a closure which defers to [read](ReadableExt::read).
    #[doc(hidden)]
    fn deref_impl(&self) -> &(impl Fn() -> Self::Target + use<Self>)
    where
        Self: Sized,
        Self::Target: Clone + 'static,
    {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        /// Helper function to transmute `src` from a `&Source` into a `&Donor`.
        /// Unlike [transmute](core::mem::transmute), this method uses a reference to
        /// the `Donor` value to allow inferring the destination type.
        /// This allows transmuting into an unnameable type (e.g., a closure).
        ///
        /// Note that the lifetime of `src`, `'a`, is unmodified by this operation.
        /// This can only be used to change the type a reference points to; it **cannot**
        /// extend the lifetime of a reference.
        ///
        /// # Safety
        ///
        /// **All** safety invariants of [transmute](core::mem::transmute) **must** be upheld by the
        /// caller of this function.
        const unsafe fn transmute_ref_by_value<'a, Source, Donor>(
            src: &'a Source,
            _: &Donor,
        ) -> &'a Donor {
            // SAFETY: Caller is responsible for upholding all invariants of transmute.
            unsafe { ::core::mem::transmute::<&'a Source, &Donor>(src) }
        }

        // The real closure that we will never use.
        let uninit_closure = const {
            // First we create a closure that captures something with the same
            // in memory layout as Self (MaybeUninit<Self>).
            let uninit_callable = ::core::mem::MaybeUninit::<Self>::uninit();

            // Then move that value into the closure.
            let uninit_closure = move || {
                // SAFETY: Initialization comes from transposing `self` into the position of `uninit_callable`.
                let this: &Self = unsafe { uninit_callable.assume_init_ref() };
                Self::read(this).clone()
            };

            // We assume that the closure now has a in memory layout of Self.
            // const-compatible alternative to:
            // assert_eq!(::core::alloc::Layout::new::<Self>(), ::core::alloc::Layout::for_value(&uninit_closure));
            {
                // FIXME: PartialEq::ne is not stable in const contexts, so must manually
                //        compare all components of `Layout`.
                let layout_self = ::core::alloc::Layout::new::<Self>();
                let layout_closure = ::core::alloc::Layout::for_value(&uninit_closure);

                if layout_self.align() != layout_closure.align()
                    || layout_self.size() != layout_closure.size()
                {
                    // This panic will be cause evaluation of the const block to fail at compile time
                    // if the assumption above is not valid.
                    panic!("assumed layout of closures capturing a single value proven false!");
                }
            }

            uninit_closure
        };

        // We transmute self into a reference to the closure.
        // SAFETY: uninit_closure is ensured to have the same Layout as self by the above assertion,
        //         and is known to contain a single value of type Self.
        //         Therefore, uninit_closure must be equivalent to self.
        unsafe { transmute_ref_by_value::<Self, _>(self, &uninit_closure) }
    }
}

impl<R: Readable + ?Sized> ReadableExt for R {}

/// An extension trait for `Readable` types that can be boxed into a trait object.
pub trait ReadableBoxExt: Readable<Storage = UnsyncStorage> {
    /// Box the readable value into a trait object. This is useful for passing around readable values without knowing their concrete type.
    fn boxed(self) -> ReadSignal<Self::Target>
    where
        Self: Sized + 'static,
    {
        ReadSignal::new(self)
    }
}
impl<R: Readable<Storage = UnsyncStorage> + ?Sized> ReadableBoxExt for R {}

/// An extension trait for `Readable<Vec<T>>` that provides some convenience methods.
pub trait ReadableVecExt<T>: Readable<Target = Vec<T>> {
    /// Returns the length of the inner vector.
    #[track_caller]
    fn len(&self) -> usize
    where
        T: 'static,
    {
        self.with(|v| v.len())
    }

    /// Returns true if the inner vector is empty.
    #[track_caller]
    fn is_empty(&self) -> bool
    where
        T: 'static,
    {
        self.with(|v| v.is_empty())
    }

    /// Get the first element of the inner vector.
    #[track_caller]
    fn first(&self) -> Option<ReadableRef<'_, Self, T>>
    where
        T: 'static,
    {
        <Self::Storage as AnyStorage>::try_map(self.read(), |v| v.first())
    }

    /// Get the last element of the inner vector.
    #[track_caller]
    fn last(&self) -> Option<ReadableRef<'_, Self, T>>
    where
        T: 'static,
    {
        <Self::Storage as AnyStorage>::try_map(self.read(), |v| v.last())
    }

    /// Get the element at the given index of the inner vector.
    #[track_caller]
    fn get(&self, index: usize) -> Option<ReadableRef<'_, Self, T>>
    where
        T: 'static,
    {
        <Self::Storage as AnyStorage>::try_map(self.read(), |v| v.get(index))
    }

    /// Get an iterator over the values of the inner vector.
    #[track_caller]
    fn iter(&self) -> ReadableValueIterator<'_, Self>
    where
        Self: Sized,
    {
        ReadableValueIterator {
            index: 0,
            value: self,
        }
    }
}

/// An iterator over the values of a `Readable<Vec<T>>`.
pub struct ReadableValueIterator<'a, R> {
    index: usize,
    value: &'a R,
}

impl<'a, T: 'static, R: Readable<Target = Vec<T>>> Iterator for ReadableValueIterator<'a, R> {
    type Item = ReadableRef<'a, R, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.value.get(index)
    }
}

impl<T, R> ReadableVecExt<T> for R where R: Readable<Target = Vec<T>> {}

/// An extension trait for `Readable<Option<T>>` that provides some convenience methods.
pub trait ReadableOptionExt<T>: Readable<Target = Option<T>> {
    /// Unwraps the inner value and clones it.
    #[track_caller]
    fn unwrap(&self) -> T
    where
        T: Clone + 'static,
    {
        self.as_ref().unwrap().clone()
    }

    /// Attempts to read the inner value of the Option.
    #[track_caller]
    fn as_ref(&self) -> Option<ReadableRef<'_, Self, T>>
    where
        T: 'static,
    {
        <Self::Storage as AnyStorage>::try_map(self.read(), |v| v.as_ref())
    }
}

impl<T, R> ReadableOptionExt<T> for R where R: Readable<Target = Option<T>> {}

/// An extension trait for `Readable<Option<T>>` that provides some convenience methods.
pub trait ReadableResultExt<T, E>: Readable<Target = Result<T, E>> {
    /// Unwraps the inner value and clones it.
    #[track_caller]
    fn unwrap(&self) -> T
    where
        T: Clone + 'static,
        E: 'static,
    {
        self.as_ref()
            .unwrap_or_else(|_| panic!("Tried to unwrap a Result that was an error"))
            .clone()
    }

    /// Attempts to read the inner value of the Option.
    #[track_caller]
    fn as_ref(&self) -> Result<ReadableRef<'_, Self, T>, ReadableRef<'_, Self, E>>
    where
        T: 'static,
        E: 'static,
    {
        <Self::Storage as AnyStorage>::try_map(self.read(), |v| v.as_ref().ok()).ok_or(
            <Self::Storage as AnyStorage>::map(self.read(), |v| v.as_ref().err().unwrap()),
        )
    }
}

impl<T, E, R> ReadableResultExt<T, E> for R where R: Readable<Target = Result<T, E>> {}

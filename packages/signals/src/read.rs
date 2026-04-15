use std::collections::{HashMap, HashSet};
use std::{
    mem::MaybeUninit,
    ops::{Deref, Index},
};

use crate::{ext_methods, MappedSignal, ReadSignal};
use dioxus_core::Subscribers;
use generational_box::{AnyStorage, UnsyncStorage};

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
    ///         for index in 0..list.read().len() {
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

    /// SAFETY: You must call this function directly with `self` as the argument.
    /// This function relies on the size of the object you return from the deref
    /// being the same as the object you pass in
    #[doc(hidden)]
    unsafe fn deref_impl<'a>(&self) -> &'a dyn Fn() -> Self::Target
    where
        Self: Sized + 'a,
        Self::Target: Clone + 'static,
    {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || Self::read(unsafe { &*uninit_callable.as_ptr() }).clone();

        // Check that the size of the closure is the same as the size of Self in case the compiler changed the layout of the closure.
        let size_of_closure = std::mem::size_of_val(&uninit_closure);
        assert_eq!(size_of_closure, std::mem::size_of::<Self>());

        // Then cast the lifetime of the closure to the lifetime of &self.
        fn cast_lifetime<'a, T>(_a: &T, b: &'a T) -> &'a T {
            b
        }
        let reference_to_closure = cast_lifetime(
            {
                // The real closure that we will never use.
                &uninit_closure
            },
            #[allow(clippy::missing_transmute_annotations)]
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
            unsafe {
                std::mem::transmute(self)
            },
        );

        // Cast the closure to a trait object.
        reference_to_closure as &_
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

// `ReadableVecExt` / `ReadableOptionExt` / `ReadableResultExt` have been
// removed. Their convenience methods (`len`, `is_empty`, `unwrap`, …) are now
// available uniformly via the `Project*` traits from `dioxus_stores` on any
// type that implements `Project` (Store, SelectorScope, LensOnly, Signal,
// ReadSignal, and WriteSignal). On other raw `Readable` types, use `.read()`
// or wrap them in `LensOnly::new(...)` to get the shape-trait methods.

/// An extension trait for [`Readable<String>`] that provides some convenience methods.
pub trait ReadableStringExt: Readable<Target = String> {
    ext_methods! {
        /// Check the capacity of the string.
        fn capacity(&self) -> usize = String::capacity;
    }
}

impl<W> ReadableStringExt for W where W: Readable<Target = String> {}

/// An extension trait for [`Readable<String>`] and [`Readable<str>`] that provides some convenience methods.
pub trait ReadableStrExt: Readable<Target: Deref<Target = str> + 'static> {
    ext_methods! {
        /// Check if the string is empty.
        fn is_empty(&self) -> bool = |s: &Self::Target| s.deref().is_empty();

        /// Get the length of the string.
        fn len(&self) -> usize = |s: &Self::Target| s.deref().len();

        /// Check if the string contains the given pattern.
        fn contains(&self, pat: &str) -> bool = |s: &Self::Target, pat| s.deref().contains(pat);
    }
}

impl<W> ReadableStrExt for W where W: Readable<Target: Deref<Target = str> + 'static> {}

// `ReadableHashMapExt` removed — use `ProjectHashMap` on a `Store<HashMap<…>>`,
// or `.read().len()` on a raw Readable.

/// An extension trait for [`Readable<HashSet<V, H>>`] that provides some convenience methods.
pub trait ReadableHashSetExt<V: 'static, H: 'static>: Readable<Target = HashSet<V, H>> {
    ext_methods! {
        /// Check if the hashset is empty.
        fn is_empty(&self) -> bool = HashSet::is_empty;

        /// Get the length of the hashset.
        fn len(&self) -> usize = HashSet::len;

        /// Get the capacity of the hashset.
        fn capacity(&self) -> usize = HashSet::capacity;
    }

    /// Check if the hashset contains the given value.
    #[track_caller]
    fn contains(&self, value: &V) -> bool
    where
        V: std::hash::Hash + Eq,
        H: std::hash::BuildHasher,
    {
        self.with(|v| v.contains(value))
    }
}

impl<V: 'static, H: 'static, R> ReadableHashSetExt<V, H> for R where
    R: Readable<Target = HashSet<V, H>>
{
}

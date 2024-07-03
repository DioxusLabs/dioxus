use std::{mem::MaybeUninit, ops::Index, rc::Rc};

use generational_box::AnyStorage;

use crate::MappedSignal;

/// A reference to a value that can be read from.
#[allow(type_alias_bounds)]
pub type ReadableRef<'a, T: Readable, O = <T as Readable>::Target> =
    <T::Storage as AnyStorage>::Ref<'a, O>;

/// A trait for states that can be read from like [`crate::Signal`], [`crate::GlobalSignal`], or [`crate::ReadOnlySignal`]. You may choose to accept this trait as a parameter instead of the concrete type to allow for more flexibility in your API. For example, instead of creating two functions, one that accepts a [`crate::Signal`] and one that accepts a [`crate::GlobalSignal`], you can create one function that accepts a [`Readable`] type.
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
///     // Since we defined the function in terms of the readable trait, we can use it with any readable type (Signal, GlobalSignal, ReadOnlySignal, etc)
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
    type Target: ?Sized + 'static;

    /// The type of the storage this readable uses.
    type Storage: AnyStorage;

    /// Map the readable type to a new type. This lets you provide a view into a readable type without needing to clone the inner value.
    ///
    /// Anything that subscribes to the readable value will be rerun whenever the original value changes, even if the view does not change. If you want to memorize the view, you can use a [`crate::Memo`] instead.
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
    /// fn Item(item: MappedSignal<i32>) -> Element {
    ///     rsx! {
    ///         div { "Item: {item}" }
    ///     }
    /// }
    /// ```
    fn map<O>(self, f: impl Fn(&Self::Target) -> &O + 'static) -> MappedSignal<O, Self::Storage>
    where
        Self: Clone + Sized + 'static,
    {
        let mapping = Rc::new(f);
        let try_read = Rc::new({
            let self_ = self.clone();
            let mapping = mapping.clone();
            move || {
                self_
                    .try_read_unchecked()
                    .map(|ref_| <Self::Storage as AnyStorage>::map(ref_, |r| mapping(r)))
            }
        })
            as Rc<
                dyn Fn() -> Result<ReadableRef<'static, Self, O>, generational_box::BorrowError>
                    + 'static,
            >;
        let peek = Rc::new(move || {
            <Self::Storage as AnyStorage>::map(self.peek_unchecked(), |r| mapping(r))
        }) as Rc<dyn Fn() -> ReadableRef<'static, Self, O> + 'static>;
        MappedSignal::new(try_read, peek)
    }

    /// Get the current value of the state. If this is a signal, this will subscribe the current scope to the signal.
    /// If the value has been dropped, this will panic. Calling this on a Signal is the same as
    /// using the signal() syntax to read and subscribe to its value
    #[track_caller]
    fn read(&self) -> ReadableRef<Self> {
        self.try_read().unwrap()
    }

    /// Try to get the current value of the state. If this is a signal, this will subscribe the current scope to the signal.
    #[track_caller]
    fn try_read(&self) -> Result<ReadableRef<Self>, generational_box::BorrowError> {
        self.try_read_unchecked()
            .map(Self::Storage::downcast_lifetime_ref)
    }

    /// Try to get a reference to the value without checking the lifetime. This will subscribe the current scope to the signal.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>;

    /// Get a reference to the value without checking the lifetime. This will subscribe the current scope to the signal.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    #[track_caller]
    fn read_unchecked(&self) -> ReadableRef<'static, Self> {
        self.try_read_unchecked().unwrap()
    }

    /// Get the current value of the signal without checking the lifetime. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    fn peek_unchecked(&self) -> ReadableRef<'static, Self>;

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
    ///         let click_count = count.peek();
    ///         tracing::info!("Click count: {click_count:?}");
    ///         count() * 2
    ///     });
    ///     rsx! {
    ///         div { "Count: {count}" }
    ///         div { "Doubled: {doubled}" }
    ///         button {
    ///             onclick: move |_| {
    ///                 event_source.set(Some("Click me button"));
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
    fn peek(&self) -> ReadableRef<Self> {
        Self::Storage::downcast_lifetime_ref(self.peek_unchecked())
    }

    /// Clone the inner value and return it. If the value has been dropped, this will panic.
    #[track_caller]
    fn cloned(&self) -> Self::Target
    where
        Self::Target: Clone,
    {
        self.read().clone()
    }

    /// Run a function with a reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn with<O>(&self, f: impl FnOnce(&Self::Target) -> O) -> O {
        f(&*self.read())
    }

    /// Run a function with a reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn with_peek<O>(&self, f: impl FnOnce(&Self::Target) -> O) -> O {
        f(&*self.peek())
    }

    /// Index into the inner value and return a reference to the result. If the value has been dropped or the index is invalid, this will panic.
    #[track_caller]
    fn index<I>(&self, index: I) -> ReadableRef<Self, <Self::Target as std::ops::Index<I>>::Output>
    where
        Self::Target: std::ops::Index<I>,
    {
        <Self::Storage as AnyStorage>::map(self.read(), |v| v.index(index))
    }

    #[doc(hidden)]
    fn deref_impl<'a>(&self) -> &'a dyn Fn() -> Self::Target
    where
        Self: Sized + 'a,
        Self::Target: Clone,
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

/// An extension trait for Readable<Vec<T>> that provides some convenience methods.
pub trait ReadableVecExt<T: 'static>: Readable<Target = Vec<T>> {
    /// Returns the length of the inner vector.
    #[track_caller]
    fn len(&self) -> usize {
        self.with(|v| v.len())
    }

    /// Returns true if the inner vector is empty.
    #[track_caller]
    fn is_empty(&self) -> bool {
        self.with(|v| v.is_empty())
    }

    /// Get the first element of the inner vector.
    #[track_caller]
    fn first(&self) -> Option<ReadableRef<Self, T>> {
        <Self::Storage as AnyStorage>::try_map(self.read(), |v| v.first())
    }

    /// Get the last element of the inner vector.
    #[track_caller]
    fn last(&self) -> Option<ReadableRef<Self, T>> {
        <Self::Storage as AnyStorage>::try_map(self.read(), |v| v.last())
    }

    /// Get the element at the given index of the inner vector.
    #[track_caller]
    fn get(&self, index: usize) -> Option<ReadableRef<Self, T>> {
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

impl<T, R> ReadableVecExt<T> for R
where
    T: 'static,
    R: Readable<Target = Vec<T>>,
{
}

/// An extension trait for Readable<Option<T>> that provides some convenience methods.
pub trait ReadableOptionExt<T: 'static>: Readable<Target = Option<T>> {
    /// Unwraps the inner value and clones it.
    #[track_caller]
    fn unwrap(&self) -> T
    where
        T: Clone,
    {
        self.as_ref().unwrap().clone()
    }

    /// Attempts to read the inner value of the Option.
    #[track_caller]
    fn as_ref(&self) -> Option<ReadableRef<Self, T>> {
        <Self::Storage as AnyStorage>::try_map(self.read(), |v| v.as_ref())
    }
}

impl<T, R> ReadableOptionExt<T> for R
where
    T: 'static,
    R: Readable<Target = Option<T>>,
{
}

/// An extension trait for Readable<Option<T>> that provides some convenience methods.
pub trait ReadableResultExt<T: 'static, E: 'static>: Readable<Target = Result<T, E>> {
    /// Unwraps the inner value and clones it.
    #[track_caller]
    fn unwrap(&self) -> T
    where
        T: Clone,
    {
        self.as_ref()
            .unwrap_or_else(|_| panic!("Tried to unwrap a Result that was an error"))
            .clone()
    }

    /// Attempts to read the inner value of the Option.
    #[track_caller]
    fn as_ref(&self) -> Result<ReadableRef<Self, T>, ReadableRef<Self, E>> {
        <Self::Storage as AnyStorage>::try_map(self.read(), |v| v.as_ref().ok()).ok_or(
            <Self::Storage as AnyStorage>::map(self.read(), |v| v.as_ref().err().unwrap()),
        )
    }
}

impl<T, E, R> ReadableResultExt<T, E> for R
where
    T: 'static,
    E: 'static,
    R: Readable<Target = Result<T, E>>,
{
}

use std::{mem::MaybeUninit, ops::Index, rc::Rc};

use generational_box::AnyStorage;

use crate::MappedSignal;

/// A reference to a value that can be read from.
#[allow(type_alias_bounds)]
pub type ReadableRef<T: Readable, O = <T as Readable>::Target> = <T::Storage as AnyStorage>::Ref<O>;

/// A trait for states that can be read from like [`crate::Signal`], [`crate::GlobalSignal`], or [`crate::ReadOnlySignal`]. You may choose to accept this trait as a parameter instead of the concrete type to allow for more flexibility in your API. For example, instead of creating two functions, one that accepts a [`crate::Signal`] and one that accepts a [`crate::GlobalSignal`], you can create one function that accepts a [`Readable`] type.
pub trait Readable {
    /// The target type of the reference.
    type Target: ?Sized + 'static;

    /// The type of the storage this readable uses.
    type Storage: AnyStorage;

    /// Map the readable type to a new type.
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
                    .try_read()
                    .map(|ref_| <Self::Storage as AnyStorage>::map(ref_, |r| mapping(r)))
            }
        })
            as Rc<
                dyn Fn() -> Result<ReadableRef<Self, O>, generational_box::BorrowError> + 'static,
            >;
        let peek = Rc::new(move || <Self::Storage as AnyStorage>::map(self.peek(), |r| mapping(r)))
            as Rc<dyn Fn() -> ReadableRef<Self, O> + 'static>;
        MappedSignal::new(try_read, peek)
    }

    /// Get the current value of the state. If this is a signal, this will subscribe the current scope to the signal. If the value has been dropped, this will panic.
    #[track_caller]
    fn read(&self) -> ReadableRef<Self> {
        self.try_read().unwrap()
    }

    /// Try to get the current value of the state. If this is a signal, this will subscribe the current scope to the signal. If the value has been dropped, this will panic.
    #[track_caller]
    fn try_read(&self) -> Result<ReadableRef<Self>, generational_box::BorrowError>;

    /// Get the current value of the state without subscribing to updates. If the value has been dropped, this will panic.
    fn peek(&self) -> ReadableRef<Self>;

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
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
            unsafe { std::mem::transmute(self) },
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
    type Item = ReadableRef<R, T>;

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

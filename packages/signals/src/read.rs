use std::ops::Deref;

/// A trait for utilities around a mutable reference
pub trait ReadableRef {
    /// The type of the reference.
    type Ref<R: ?Sized + 'static>: Deref<Target = R>;

    /// Map the reference to a new type.
    fn map_ref<I, U: ?Sized + 'static, F: FnOnce(&I) -> &U>(
        ref_: Self::Ref<I>,
        f: F,
    ) -> Self::Ref<U>;

    /// Try to map the reference to a new type.
    fn try_map_ref<I, U: ?Sized + 'static, F: FnOnce(&I) -> Option<&U>>(
        ref_: Self::Ref<I>,
        f: F,
    ) -> Option<Self::Ref<U>>;
}

/// A trait for states that can be read from like [`crate::Signal`], [`crate::GlobalSignal`], or [`crate::ReadOnlySignal`]. You may choose to accept this trait as a parameter instead of the concrete type to allow for more flexibility in your API. For example, instead of creating two functions, one that accepts a [`crate::Signal`] and one that accepts a [`crate::GlobalSignal`], you can create one function that accepts a [`Readable`] type.
pub trait Readable<T: 'static>: ReadableRef {
    /// Get the current value of the state. If this is a signal, this will subscribe the current scope to the signal. If the value has been dropped, this will panic.
    fn read(&self) -> Self::Ref<T>;

    /// Get the current value of the state without subscribing to updates. If the value has been dropped, this will panic.
    fn peek(&self) -> Self::Ref<T>;

    /// Clone the inner value and return it. If the value has been dropped, this will panic.
    #[track_caller]
    fn cloned(&self) -> T
    where
        T: Clone,
    {
        self.read().clone()
    }

    /// Run a function with a reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        f(&*self.read())
    }

    /// Run a function with a reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn with_peek<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        f(&*self.peek())
    }

    /// Index into the inner value and return a reference to the result. If the value has been dropped or the index is invalid, this will panic.
    #[track_caller]
    fn index<I>(&self, index: I) -> Self::Ref<T::Output>
    where
        T: std::ops::Index<I>,
    {
        Self::map_ref(self.read(), |v| v.index(index))
    }
}

/// An extension trait for Readable<Vec<T>> that provides some convenience methods.
pub trait ReadableVecExt<T: 'static>: Readable<Vec<T>> {
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
    fn first(&self) -> Option<Self::Ref<T>> {
        Self::try_map_ref(self.read(), |v| v.first())
    }

    /// Get the last element of the inner vector.
    #[track_caller]
    fn last(&self) -> Option<Self::Ref<T>> {
        Self::try_map_ref(self.read(), |v| v.last())
    }

    /// Get the element at the given index of the inner vector.
    #[track_caller]
    fn get(&self, index: usize) -> Option<Self::Ref<T>> {
        Self::try_map_ref(self.read(), |v| v.get(index))
    }

    /// Get an iterator over the values of the inner vector.
    #[track_caller]
    fn iter(&self) -> ReadableValueIterator<T, Self>
    where
        Self: Sized + Clone,
    {
        ReadableValueIterator {
            index: 0,
            value: self.clone(),
            phantom: std::marker::PhantomData,
        }
    }
}

/// An iterator over the values of a `Readable<Vec<T>>`.
pub struct ReadableValueIterator<T, R> {
    index: usize,
    value: R,
    phantom: std::marker::PhantomData<T>,
}

impl<T: 'static, R: Readable<Vec<T>>> Iterator for ReadableValueIterator<T, R> {
    type Item = R::Ref<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.value.get(index)
    }
}

impl<T, R> ReadableVecExt<T> for R
where
    T: 'static,
    R: Readable<Vec<T>>,
{
}

/// An extension trait for Readable<Option<T>> that provides some convenience methods.
pub trait ReadableOptionExt<T: 'static>: Readable<Option<T>> {
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
    fn as_ref(&self) -> Option<Self::Ref<T>> {
        Self::try_map_ref(self.read(), |v| v.as_ref())
    }
}

impl<T, R> ReadableOptionExt<T> for R
where
    T: 'static,
    R: Readable<Option<T>>,
{
}

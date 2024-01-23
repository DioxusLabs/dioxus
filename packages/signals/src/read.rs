use std::ops::Deref;

pub trait Readable<T: 'static> {
    type Ref<R: ?Sized + 'static>: Deref<Target = R>;

    fn map_ref<I, U: ?Sized, F: FnOnce(&I) -> &U>(ref_: Self::Ref<I>, f: F) -> Self::Ref<U>;

    fn try_map_ref<I, U: ?Sized, F: FnOnce(&I) -> Option<&U>>(
        ref_: Self::Ref<I>,
        f: F,
    ) -> Option<Self::Ref<U>>;

    fn read(&self) -> Self::Ref<T>;

    fn peek(&self) -> Self::Ref<T>;

    fn cloned(&self) -> T
    where
        T: Clone,
    {
        self.read().clone()
    }

    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        f(&*self.read())
    }

    fn with_peek<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        f(&*self.peek())
    }

    /// Index into the inner value and return a reference to the result.
    #[track_caller]
    fn index<I>(&self, index: I) -> Self::Ref<T::Output>
    where
        T: std::ops::Index<I>,
    {
        Self::map_ref(self.read(), |v| v.index(index))
    }
}

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
}

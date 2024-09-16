use std::ops::{DerefMut, IndexMut};

use crate::read::Readable;

/// A reference to a value that can be read from.
#[allow(type_alias_bounds)]
pub type WritableRef<'a, T: Writable, O = <T as Readable>::Target> = T::Mut<'a, O>;

/// A trait for states that can be written to like [`crate::Signal`]. You may choose to accept this trait as a parameter instead of the concrete type to allow for more flexibility in your API.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// enum MyEnum {
///     String(String),
///     Number(i32),
/// }
///
/// fn MyComponent(mut count: Signal<MyEnum>) -> Element {
///     rsx! {
///         button {
///             onclick: move |_| {
///                 // You can use any methods from the Writable trait on Signals
///                 match &mut *count.write() {
///                     MyEnum::String(s) => s.push('a'),
///                     MyEnum::Number(n) => *n += 1,
///                 }
///             },
///             "Add value"
///         }
///     }
/// }
/// ```
pub trait Writable: Readable {
    /// The type of the reference.
    type Mut<'a, R: ?Sized + 'static>: DerefMut<Target = R>;

    /// Map the reference to a new type.
    fn map_mut<I: ?Sized, U: ?Sized, F: FnOnce(&mut I) -> &mut U>(
        ref_: Self::Mut<'_, I>,
        f: F,
    ) -> Self::Mut<'_, U>;

    /// Try to map the reference to a new type.
    fn try_map_mut<I: ?Sized, U: ?Sized, F: FnOnce(&mut I) -> Option<&mut U>>(
        ref_: Self::Mut<'_, I>,
        f: F,
    ) -> Option<Self::Mut<'_, U>>;

    /// Downcast a mutable reference in a RefMut to a more specific lifetime
    ///
    /// This function enforces the variance of the lifetime parameter `'a` in Ref.
    fn downcast_lifetime_mut<'a: 'b, 'b, T: ?Sized + 'static>(
        mut_: Self::Mut<'a, T>,
    ) -> Self::Mut<'b, T>;

    /// Get a mutable reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn write(&mut self) -> WritableRef<'_, Self> {
        self.try_write().unwrap()
    }

    /// Try to get a mutable reference to the value.
    #[track_caller]
    fn try_write(&mut self) -> Result<WritableRef<'_, Self>, generational_box::BorrowMutError> {
        self.try_write_unchecked().map(Self::downcast_lifetime_mut)
    }

    /// Try to get a mutable reference to the value without checking the lifetime. This will update any subscribers.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError>;

    /// Get a mutable reference to the value without checking the lifetime. This will update any subscribers.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    #[track_caller]
    fn write_unchecked(&self) -> WritableRef<'static, Self> {
        self.try_write_unchecked().unwrap()
    }

    /// Run a function with a mutable reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn with_mut<O>(&mut self, f: impl FnOnce(&mut Self::Target) -> O) -> O {
        f(&mut *self.write())
    }

    /// Set the value of the signal. This will trigger an update on all subscribers.
    #[track_caller]
    fn set(&mut self, value: Self::Target)
    where
        Self::Target: Sized,
    {
        *self.write() = value;
    }

    /// Invert the boolean value of the signal. This will trigger an update on all subscribers.
    #[track_caller]
    fn toggle(&mut self)
    where
        Self::Target: std::ops::Not<Output = Self::Target> + Clone,
    {
        self.set(!self.cloned());
    }

    /// Index into the inner value and return a reference to the result.
    #[track_caller]
    fn index_mut<I>(
        &mut self,
        index: I,
    ) -> WritableRef<'_, Self, <Self::Target as std::ops::Index<I>>::Output>
    where
        Self::Target: std::ops::IndexMut<I>,
    {
        Self::map_mut(self.write(), |v| v.index_mut(index))
    }

    /// Takes the value out of the Signal, leaving a Default in its place.
    #[track_caller]
    fn take(&mut self) -> Self::Target
    where
        Self::Target: Default,
    {
        self.with_mut(std::mem::take)
    }

    /// Replace the value in the Signal, returning the old value.
    #[track_caller]
    fn replace(&mut self, value: Self::Target) -> Self::Target
    where
        Self::Target: Sized,
    {
        self.with_mut(|v| std::mem::replace(v, value))
    }
}

/// An extension trait for [`Writable<Option<T>>`]` that provides some convenience methods.
pub trait WritableOptionExt<T: 'static>: Writable<Target = Option<T>> {
    /// Gets the value out of the Option, or inserts the given value if the Option is empty.
    #[track_caller]
    fn get_or_insert(&mut self, default: T) -> WritableRef<'_, Self, T> {
        self.get_or_insert_with(|| default)
    }

    /// Gets the value out of the Option, or inserts the value returned by the given function if the Option is empty.
    #[track_caller]
    fn get_or_insert_with(&mut self, default: impl FnOnce() -> T) -> WritableRef<'_, Self, T> {
        let is_none = self.read().is_none();
        if is_none {
            self.with_mut(|v| *v = Some(default()));
            Self::map_mut(self.write(), |v| v.as_mut().unwrap())
        } else {
            Self::map_mut(self.write(), |v| v.as_mut().unwrap())
        }
    }

    /// Attempts to write the inner value of the Option.
    #[track_caller]
    fn as_mut(&mut self) -> Option<WritableRef<'_, Self, T>> {
        Self::try_map_mut(self.write(), |v: &mut Option<T>| v.as_mut())
    }
}

impl<T, W> WritableOptionExt<T> for W
where
    T: 'static,
    W: Writable<Target = Option<T>>,
{
}

/// An extension trait for [`Writable<Vec<T>>`] that provides some convenience methods.
pub trait WritableVecExt<T: 'static>: Writable<Target = Vec<T>> {
    /// Pushes a new value to the end of the vector.
    #[track_caller]
    fn push(&mut self, value: T) {
        self.with_mut(|v| v.push(value))
    }

    /// Pops the last value from the vector.
    #[track_caller]
    fn pop(&mut self) -> Option<T> {
        self.with_mut(|v| v.pop())
    }

    /// Inserts a new value at the given index.
    #[track_caller]
    fn insert(&mut self, index: usize, value: T) {
        self.with_mut(|v| v.insert(index, value))
    }

    /// Removes the value at the given index.
    #[track_caller]
    fn remove(&mut self, index: usize) -> T {
        self.with_mut(|v| v.remove(index))
    }

    /// Clears the vector, removing all values.
    #[track_caller]
    fn clear(&mut self) {
        self.with_mut(|v| v.clear())
    }

    /// Extends the vector with the given iterator.
    #[track_caller]
    fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
        self.with_mut(|v| v.extend(iter))
    }

    /// Truncates the vector to the given length.
    #[track_caller]
    fn truncate(&mut self, len: usize) {
        self.with_mut(|v| v.truncate(len))
    }

    /// Swaps two values in the vector.
    #[track_caller]
    fn swap_remove(&mut self, index: usize) -> T {
        self.with_mut(|v| v.swap_remove(index))
    }

    /// Retains only the values that match the given predicate.
    #[track_caller]
    fn retain(&mut self, f: impl FnMut(&T) -> bool) {
        self.with_mut(|v| v.retain(f))
    }

    /// Splits the vector into two at the given index.
    #[track_caller]
    fn split_off(&mut self, at: usize) -> Vec<T> {
        self.with_mut(|v| v.split_off(at))
    }

    /// Try to mutably get an element from the vector.
    #[track_caller]
    fn get_mut(&mut self, index: usize) -> Option<WritableRef<'_, Self, T>> {
        Self::try_map_mut(self.write(), |v: &mut Vec<T>| v.get_mut(index))
    }

    /// Gets an iterator over the values of the vector.
    #[track_caller]
    fn iter_mut(&mut self) -> WritableValueIterator<'_, Self>
    where
        Self: Sized + Clone,
    {
        WritableValueIterator {
            index: 0,
            value: self,
        }
    }
}

/// An iterator over the values of a [`Writable<Vec<T>>`].
pub struct WritableValueIterator<'a, R> {
    index: usize,
    value: &'a mut R,
}

impl<'a, T: 'static, R: Writable<Target = Vec<T>>> Iterator for WritableValueIterator<'a, R> {
    type Item = WritableRef<'a, R, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        R::try_map_mut(
            self.value.try_write_unchecked().unwrap(),
            |v: &mut Vec<T>| v.get_mut(index),
        )
        .map(R::downcast_lifetime_mut)
    }
}

impl<T, W> WritableVecExt<T> for W
where
    T: 'static,
    W: Writable<Target = Vec<T>>,
{
}

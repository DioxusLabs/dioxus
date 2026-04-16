//! Storage-aware mutable reference wrapper used across the reactive stack.
//!
//! `WriteLock` pairs a storage-provided mutable reference (`S::Mut`) with
//! optional metadata `D` — typically a drop guard that notifies subscribers
//! when the write ends. The type is storage-agnostic, so it lives in the
//! foundational crate where any consumer can use it.

use std::ops::{Deref, DerefMut};

use crate::{AnyStorage, UnsyncStorage};

/// A storage-backed mutable reference with optional metadata.
///
/// ## Generics
/// - `T` is the current type of the write
/// - `S` is the storage type of the signal. This type determines if the
///   reference is local to the current thread, or shared across threads.
/// - `D` is the additional data associated with the write reference. Reactive
///   crates use this to track when the write is dropped and notify
///   subscribers.
pub struct WriteLock<'a, T: ?Sized + 'a, S: AnyStorage = UnsyncStorage, D = ()> {
    write: S::Mut<'a, T>,
    data: D,
}

impl<'a, T: ?Sized, S: AnyStorage> WriteLock<'a, T, S> {
    /// Create a new write reference.
    pub fn new(write: S::Mut<'a, T>) -> Self {
        Self { write, data: () }
    }
}

impl<'a, T: ?Sized, S: AnyStorage, D> WriteLock<'a, T, S, D> {
    /// Create a new write reference with additional data.
    pub fn new_with_metadata(write: S::Mut<'a, T>, data: D) -> Self {
        Self { write, data }
    }

    /// Get the inner value of the write reference.
    pub fn into_inner(self) -> S::Mut<'a, T> {
        self.write
    }

    /// Get the additional data associated with the write reference.
    pub fn data(&self) -> &D {
        &self.data
    }

    /// Split into the inner value and the additional data.
    pub fn into_parts(self) -> (S::Mut<'a, T>, D) {
        (self.write, self.data)
    }

    /// Map the metadata of the write reference to a new type.
    pub fn map_metadata<O>(self, f: impl FnOnce(D) -> O) -> WriteLock<'a, T, S, O> {
        WriteLock {
            write: self.write,
            data: f(self.data),
        }
    }

    /// Map the mutable reference to a new type.
    pub fn map<O: ?Sized>(
        myself: Self,
        f: impl FnOnce(&mut T) -> &mut O,
    ) -> WriteLock<'a, O, S, D> {
        let Self { write, data, .. } = myself;
        WriteLock {
            write: S::map_mut(write, f),
            data,
        }
    }

    /// Try to map the mutable reference to a new type.
    pub fn filter_map<O: ?Sized>(
        myself: Self,
        f: impl FnOnce(&mut T) -> Option<&mut O>,
    ) -> Option<WriteLock<'a, O, S, D>> {
        let Self { write, data, .. } = myself;
        let write = S::try_map_mut(write, f);
        write.map(|write| WriteLock { write, data })
    }

    /// Downcast the lifetime of the mutable reference.
    ///
    /// This function enforces the variance of the lifetime parameter `'a` in
    /// `Mut`. Rust typically infers this cast with a concrete type, but it
    /// cannot with a generic type.
    pub fn downcast_lifetime<'b>(mut_: Self) -> WriteLock<'b, T, S, D>
    where
        'a: 'b,
    {
        WriteLock {
            write: S::downcast_lifetime_mut(mut_.write),
            data: mut_.data,
        }
    }
}

impl<T, S, D> Deref for WriteLock<'_, T, S, D>
where
    S: AnyStorage,
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.write
    }
}

impl<T, S, D> DerefMut for WriteLock<'_, T, S, D>
where
    S: AnyStorage,
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.write
    }
}

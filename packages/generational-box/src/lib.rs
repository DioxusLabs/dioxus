#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use parking_lot::Mutex;
use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub use error::*;
pub use references::*;
pub use sync::SyncStorage;
pub use unsync::UnsyncStorage;

mod entry;
mod error;
mod references;
mod sync;
mod unsync;

/// The type erased id of a generational box.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenerationalBoxId {
    data_ptr: *const (),
    generation: u64,
}

// Safety: GenerationalBoxId is Send and Sync because there is no way to access the pointer.
unsafe impl Send for GenerationalBoxId {}
unsafe impl Sync for GenerationalBoxId {}

impl Debug for GenerationalBoxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}@{:?}", self.data_ptr, self.generation))?;
        Ok(())
    }
}

/// The core Copy state type. The generational box will be dropped when the [Owner] is dropped.
pub struct GenerationalBox<T, S: 'static = UnsyncStorage> {
    raw: GenerationalPointer<S>,
    _marker: PhantomData<T>,
}

impl<T, S: AnyStorage> Debug for GenerationalBox<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.raw.fmt(f)
    }
}

impl<T, S: Storage<T>> GenerationalBox<T, S> {
    /// Create a new generational box by leaking a value into the storage. This is useful for creating
    /// a box that needs to be manually dropped with no owners.
    #[track_caller]
    pub fn leak(value: T) -> Self {
        let location = S::claim(std::panic::Location::caller());
        location.set(value);
        Self {
            raw: location,
            _marker: PhantomData,
        }
    }

    /// Get the raw pointer to the value.
    pub fn raw_ptr(&self) -> *const () {
        self.raw.storage.data_ptr()
    }

    /// Get the id of the generational box.
    pub fn id(&self) -> GenerationalBoxId {
        self.raw.id()
    }

    /// Try to read the value. Returns an error if the value is no longer valid.
    #[track_caller]
    pub fn try_read(&self) -> Result<S::Ref<'static, T>, BorrowError> {
        self.raw.try_read()
    }

    /// Read the value. Panics if the value is no longer valid.
    #[track_caller]
    pub fn read(&self) -> S::Ref<'static, T> {
        self.try_read().unwrap()
    }

    /// Try to write the value. Returns None if the value is no longer valid.
    #[track_caller]
    pub fn try_write(&self) -> Result<S::Mut<'static, T>, BorrowMutError> {
        self.raw.try_write()
    }

    /// Write the value. Panics if the value is no longer valid.
    #[track_caller]
    pub fn write(&self) -> S::Mut<'static, T> {
        self.try_write().unwrap()
    }

    /// Set the value. Panics if the value is no longer valid.
    pub fn set(&self, value: T) {
        S::set(self.raw, value);
    }

    /// Returns true if the pointer is equal to the other pointer.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }

    /// Drop the value out of the generational box and invalidate the generational box.
    /// This will return the value if the value was taken.
    pub fn manually_drop(&self) -> Option<T>
    where
        T: 'static,
    {
        self.raw.take()
    }
}

impl<T, S> Copy for GenerationalBox<T, S> {}

impl<T, S> Clone for GenerationalBox<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A trait for a storage backing type. (RefCell, RwLock, etc.)
pub trait Storage<Data = ()>: AnyStorage + 'static {
    /// Try to read the value. Returns None if the value is no longer valid.
    fn try_read(
        location: GenerationalPointer<Self>,
    ) -> Result<Self::Ref<'static, Data>, BorrowError>;

    /// Try to write the value. Returns None if the value is no longer valid.
    fn try_write(
        location: GenerationalPointer<Self>,
    ) -> Result<Self::Mut<'static, Data>, BorrowMutError>;

    /// Set the value if the location is valid
    fn set(location: GenerationalPointer<Self>, value: Data);
}

/// A trait for any storage backing type.
pub trait AnyStorage: Default + 'static {
    /// The reference this storage type returns.
    type Ref<'a, T: ?Sized + 'static>: Deref<Target = T>;
    /// The mutable reference this storage type returns.
    type Mut<'a, T: ?Sized + 'static>: DerefMut<Target = T>;

    /// Downcast a reference in a Ref to a more specific lifetime
    ///
    /// This function enforces the variance of the lifetime parameter `'a` in Ref. Rust will typically infer this cast with a concrete type, but it cannot with a generic type.
    fn downcast_lifetime_ref<'a: 'b, 'b, T: ?Sized + 'static>(
        ref_: Self::Ref<'a, T>,
    ) -> Self::Ref<'b, T>;

    /// Downcast a mutable reference in a RefMut to a more specific lifetime
    ///
    /// This function enforces the variance of the lifetime parameter `'a` in Mut.  Rust will typically infer this cast with a concrete type, but it cannot with a generic type.
    fn downcast_lifetime_mut<'a: 'b, 'b, T: ?Sized + 'static>(
        mut_: Self::Mut<'a, T>,
    ) -> Self::Mut<'b, T>;

    /// Try to map the mutable ref.
    fn try_map_mut<T: ?Sized + 'static, U: ?Sized + 'static>(
        mut_ref: Self::Mut<'_, T>,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Self::Mut<'_, U>>;

    /// Map the mutable ref.
    fn map_mut<T: ?Sized + 'static, U: ?Sized + 'static>(
        mut_ref: Self::Mut<'_, T>,
        f: impl FnOnce(&mut T) -> &mut U,
    ) -> Self::Mut<'_, U> {
        Self::try_map_mut(mut_ref, |v| Some(f(v))).unwrap()
    }

    /// Try to map the ref.
    fn try_map<T: ?Sized, U: ?Sized + 'static>(
        ref_: Self::Ref<'_, T>,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<Self::Ref<'_, U>>;

    /// Map the ref.
    fn map<T: ?Sized, U: ?Sized + 'static>(
        ref_: Self::Ref<'_, T>,
        f: impl FnOnce(&T) -> &U,
    ) -> Self::Ref<'_, U> {
        Self::try_map(ref_, |v| Some(f(v))).unwrap()
    }

    /// Get the data pointer. No guarantees are made about the data pointer. It should only be used for debugging.
    fn data_ptr(&self) -> *const ();

    /// Recycle a memory location. This will drop the memory location and return it to the runtime.
    fn recycle(location: GenerationalPointer<Self>) -> Option<Box<dyn std::any::Any>>;

    /// Claim a new memory location. This will either create a new memory location or recycle an old one.
    fn claim(caller: &'static std::panic::Location<'static>) -> GenerationalPointer<Self>;

    /// Create a new owner. The owner will be responsible for dropping all of the generational boxes that it creates.
    fn owner() -> Owner<Self> {
        Owner(Arc::new(Mutex::new(OwnerInner {
            owned: Default::default(),
        })))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GenerationalLocation {
    /// The generation this location is associated with. Using the location after this generation is invalidated will return errors.
    generation: u64,
    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
    created_at: &'static std::panic::Location<'static>,
}

/// A pointer to a specific generational box and generation in that box.
pub struct GenerationalPointer<S: 'static = UnsyncStorage> {
    /// The storage that is backing this location
    storage: &'static S,
    /// The location of the data
    location: GenerationalLocation,
}

impl<S: AnyStorage + 'static> PartialEq for GenerationalPointer<S> {
    fn eq(&self, other: &Self) -> bool {
        self.storage.data_ptr() == other.storage.data_ptr()
            && self.location.generation == other.location.generation
    }
}

impl<S: AnyStorage + 'static> Debug for GenerationalPointer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:?}@{:?}",
            self.storage.data_ptr(),
            self.location.generation
        ))
    }
}

impl<S: 'static> Clone for GenerationalPointer<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: 'static> Copy for GenerationalPointer<S> {}

impl<S> GenerationalPointer<S> {
    fn take<T: 'static>(self) -> Option<T>
    where
        S: Storage<T>,
    {
        S::recycle(self).map(|value| *(value.downcast().unwrap()))
    }

    fn set<T>(self, value: T)
    where
        S: Storage<T>,
    {
        S::set(self, value)
    }

    #[track_caller]
    fn try_read<T>(self) -> Result<S::Ref<'static, T>, BorrowError>
    where
        S: Storage<T>,
    {
        S::try_read(self)
    }

    #[track_caller]
    fn try_write<T>(self) -> Result<S::Mut<'static, T>, BorrowMutError>
    where
        S: Storage<T>,
    {
        S::try_write(self)
    }

    fn recycle(self)
    where
        S: AnyStorage,
    {
        S::recycle(self);
    }

    fn id(&self) -> GenerationalBoxId
    where
        S: AnyStorage,
    {
        GenerationalBoxId {
            data_ptr: self.storage.data_ptr(),
            generation: self.location.generation,
        }
    }
}

struct OwnerInner<S: AnyStorage + 'static> {
    owned: Vec<GenerationalPointer<S>>,
}

impl<S: AnyStorage> Drop for OwnerInner<S> {
    fn drop(&mut self) {
        for location in self.owned.drain(..) {
            location.recycle();
        }
    }
}

/// Owner: Handles dropping generational boxes. The owner acts like a runtime lifetime guard. Any states that you create with an owner will be dropped when that owner is dropped.
pub struct Owner<S: AnyStorage + 'static = UnsyncStorage>(Arc<Mutex<OwnerInner<S>>>);

impl<S: AnyStorage> Default for Owner<S> {
    fn default() -> Self {
        S::owner()
    }
}

impl<S: AnyStorage> Clone for Owner<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S: AnyStorage> Owner<S> {
    /// Insert a value into the store. The value will be dropped when the owner is dropped.
    #[track_caller]
    pub fn insert<T: 'static>(&self, value: T) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        self.insert_with_caller(value, std::panic::Location::caller())
    }

    /// Insert a value into the store with a specific location blamed for creating the value. The value will be dropped when the owner is dropped.
    pub fn insert_with_caller<T: 'static>(
        &self,
        value: T,
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        let location = S::claim(caller);
        location.set(value);
        self.0.lock().owned.push(location);
        GenerationalBox {
            raw: location,
            _marker: PhantomData,
        }
    }

    /// Creates an invalid handle. This is useful for creating a handle that will be filled in later. If you use this before the value is filled in, you will get may get a panic or an out of date value.
    #[track_caller]
    pub fn invalid<T: 'static>(&self) -> GenerationalBox<T, S> {
        let location = S::claim(std::panic::Location::caller());
        self.0.lock().owned.push(location);
        GenerationalBox {
            raw: location,
            _marker: PhantomData,
        }
    }
}

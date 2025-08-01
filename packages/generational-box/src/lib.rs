#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use parking_lot::Mutex;
use std::{
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroU64,
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
    generation: NonZeroU64,
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
    pub fn leak(value: T, location: &'static std::panic::Location<'static>) -> Self {
        let location = S::new(value, location);
        Self {
            raw: location,
            _marker: PhantomData,
        }
    }

    /// Create a new reference counted generational box by leaking a value into the storage. This is useful for creating
    /// a box that needs to be manually dropped with no owners.
    #[track_caller]
    pub fn leak_rc(value: T, location: &'static std::panic::Location<'static>) -> Self {
        let location = S::new_rc(value, location);
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
    #[track_caller]
    pub fn set(&self, value: T)
    where
        T: 'static,
    {
        *self.write() = value;
    }

    /// Drop the value out of the generational box and invalidate the generational box.
    pub fn manually_drop(&self)
    where
        T: 'static,
    {
        self.raw.recycle();
    }

    /// Get a reference to the value
    #[track_caller]
    pub fn leak_reference(&self) -> BorrowResult<GenerationalBox<T, S>> {
        Ok(Self {
            raw: S::new_reference(self.raw)?,
            _marker: std::marker::PhantomData,
        })
    }

    /// Change this box to point to another generational box
    pub fn point_to(&self, other: GenerationalBox<T, S>) -> BorrowResult {
        S::change_reference(self.raw, other.raw)
    }
}

impl<T, S> GenerationalBox<T, S> {
    /// Returns true if the pointer is equal to the other pointer.
    pub fn ptr_eq(&self, other: &Self) -> bool
    where
        S: AnyStorage,
    {
        self.raw == other.raw
    }

    /// Try to get the location the generational box was created at. In release mode this will always return None.
    pub fn created_at(&self) -> Option<&'static std::panic::Location<'static>> {
        self.raw.location.created_at()
    }
}

impl<T, S> Copy for GenerationalBox<T, S> {}

impl<T, S> Clone for GenerationalBox<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A trait for a storage backing type. (RefCell, RwLock, etc.)
pub trait Storage<Data = ()>: AnyStorage {
    /// Try to read the value. Returns None if the value is no longer valid.
    fn try_read(pointer: GenerationalPointer<Self>) -> BorrowResult<Self::Ref<'static, Data>>;

    /// Try to write the value. Returns None if the value is no longer valid.
    fn try_write(pointer: GenerationalPointer<Self>) -> BorrowMutResult<Self::Mut<'static, Data>>;

    /// Create a new memory location. This will either create a new memory location or recycle an old one.
    fn new(
        value: Data,
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalPointer<Self>;

    /// Create a new reference counted memory location. This will either create a new memory location or recycle an old one.
    fn new_rc(
        value: Data,
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalPointer<Self>;

    /// Reference another location if the location is valid
    ///
    /// This method may return an error if the other box is no longer valid or it is already borrowed mutably.
    fn new_reference(inner: GenerationalPointer<Self>) -> BorrowResult<GenerationalPointer<Self>>;

    /// Change the reference a signal is pointing to
    ///
    /// This method may return an error if the other box is no longer valid or it is already borrowed mutably.
    fn change_reference(
        pointer: GenerationalPointer<Self>,
        rc_pointer: GenerationalPointer<Self>,
    ) -> BorrowResult;
}

/// A trait for any storage backing type.
pub trait AnyStorage: Default {
    /// The reference this storage type returns.
    type Ref<'a, T: ?Sized + 'a>: Deref<Target = T>;
    /// The mutable reference this storage type returns.
    type Mut<'a, T: ?Sized + 'a>: DerefMut<Target = T>;

    /// Downcast a reference in a Ref to a more specific lifetime
    ///
    /// This function enforces the variance of the lifetime parameter `'a` in Ref. Rust will typically infer this cast with a concrete type, but it cannot with a generic type.
    fn downcast_lifetime_ref<'a: 'b, 'b, T: ?Sized + 'a>(
        ref_: Self::Ref<'a, T>,
    ) -> Self::Ref<'b, T>;

    /// Downcast a mutable reference in a RefMut to a more specific lifetime
    ///
    /// This function enforces the variance of the lifetime parameter `'a` in Mut.  Rust will typically infer this cast with a concrete type, but it cannot with a generic type.
    fn downcast_lifetime_mut<'a: 'b, 'b, T: ?Sized + 'a>(
        mut_: Self::Mut<'a, T>,
    ) -> Self::Mut<'b, T>;

    /// Try to map the mutable ref.
    fn try_map_mut<T: ?Sized, U: ?Sized>(
        mut_ref: Self::Mut<'_, T>,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Self::Mut<'_, U>>;

    /// Map the mutable ref.
    fn map_mut<T: ?Sized, U: ?Sized>(
        mut_ref: Self::Mut<'_, T>,
        f: impl FnOnce(&mut T) -> &mut U,
    ) -> Self::Mut<'_, U> {
        Self::try_map_mut(mut_ref, |v| Some(f(v))).unwrap()
    }

    /// Try to map the ref.
    fn try_map<T: ?Sized, U: ?Sized>(
        ref_: Self::Ref<'_, T>,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<Self::Ref<'_, U>>;

    /// Map the ref.
    fn map<T: ?Sized, U: ?Sized>(
        ref_: Self::Ref<'_, T>,
        f: impl FnOnce(&T) -> &U,
    ) -> Self::Ref<'_, U> {
        Self::try_map(ref_, |v| Some(f(v))).unwrap()
    }

    /// Get the data pointer. No guarantees are made about the data pointer. It should only be used for debugging.
    fn data_ptr(&self) -> *const ();

    /// Recycle a memory location. This will drop the memory location and return it to the runtime.
    fn recycle(location: GenerationalPointer<Self>);

    /// Create a new owner. The owner will be responsible for dropping all of the generational boxes that it creates.
    fn owner() -> Owner<Self>
    where
        Self: 'static,
    {
        Owner(Arc::new(Mutex::new(OwnerInner {
            owned: Default::default(),
        })))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GenerationalLocation {
    /// The generation this location is associated with. Using the location after this generation is invalidated will return errors.
    generation: NonZeroU64,
    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
    created_at: &'static std::panic::Location<'static>,
}

impl GenerationalLocation {
    pub(crate) fn created_at(&self) -> Option<&'static std::panic::Location<'static>> {
        #[cfg(debug_assertions)]
        {
            Some(self.created_at)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
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
    pub fn insert<T>(&self, value: T) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        self.insert_with_caller(value, std::panic::Location::caller())
    }

    /// Create a new reference counted box. The box will be dropped when all references are dropped.
    #[track_caller]
    pub fn insert_rc<T>(&self, value: T) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        self.insert_rc_with_caller(value, std::panic::Location::caller())
    }

    /// Insert a value into the store with a specific location blamed for creating the value. The value will be dropped when the owner is dropped.
    pub fn insert_rc_with_caller<T>(
        &self,
        value: T,
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        let location = S::new_rc(value, caller);
        self.0.lock().owned.push(location);
        GenerationalBox {
            raw: location,
            _marker: std::marker::PhantomData,
        }
    }

    /// Insert a value into the store with a specific location blamed for creating the value. The value will be dropped when the owner is dropped.
    pub fn insert_with_caller<T>(
        &self,
        value: T,
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        let location = S::new(value, caller);
        self.0.lock().owned.push(location);
        GenerationalBox {
            raw: location,
            _marker: PhantomData,
        }
    }

    /// Create a new reference to an existing box. The reference will be dropped when the owner is dropped.
    ///
    /// This method may return an error if the other box is no longer valid or it is already borrowed mutably.
    #[track_caller]
    pub fn insert_reference<T>(
        &self,
        other: GenerationalBox<T, S>,
    ) -> BorrowResult<GenerationalBox<T, S>>
    where
        S: Storage<T>,
    {
        let location = other.leak_reference()?;
        self.0.lock().owned.push(location.raw);
        Ok(location)
    }
}

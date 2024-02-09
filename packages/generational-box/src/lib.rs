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

mod error;
mod references;
mod sync;
mod unsync;

/// The type erased id of a generational box.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenerationalBoxId {
    data_ptr: *const (),
    #[cfg(any(debug_assertions, feature = "check_generation"))]
    generation: u32,
}

// Safety: GenerationalBoxId is Send and Sync because there is no way to access the pointer.
unsafe impl Send for GenerationalBoxId {}
unsafe impl Sync for GenerationalBoxId {}

impl Debug for GenerationalBoxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        f.write_fmt(format_args!("{:?}@{:?}", self.data_ptr, self.generation))?;
        #[cfg(not(any(debug_assertions, feature = "check_generation")))]
        f.write_fmt(format_args!("{:?}", self.data_ptr))?;
        Ok(())
    }
}

/// The core Copy state type. The generational box will be dropped when the [Owner] is dropped.
pub struct GenerationalBox<T, S: 'static = UnsyncStorage> {
    raw: MemoryLocation<S>,
    #[cfg(any(debug_assertions, feature = "check_generation"))]
    generation: u32,
    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
    created_at: &'static std::panic::Location<'static>,
    _marker: PhantomData<T>,
}

impl<T: 'static, S: AnyStorage> Debug for GenerationalBox<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        f.write_fmt(format_args!(
            "{:?}@{:?}",
            self.raw.0.data.data_ptr(),
            self.generation
        ))?;
        #[cfg(not(any(debug_assertions, feature = "check_generation")))]
        f.write_fmt(format_args!("{:?}", self.raw.0.data.data_ptr()))?;
        Ok(())
    }
}

impl<T: 'static, S: Storage<T>> GenerationalBox<T, S> {
    #[inline(always)]
    pub(crate) fn validate(&self) -> bool {
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        {
            self.raw
                .0
                .generation
                .load(std::sync::atomic::Ordering::Relaxed)
                == self.generation
        }
        #[cfg(not(any(debug_assertions, feature = "check_generation")))]
        {
            true
        }
    }

    /// Get the raw pointer to the value.
    pub fn raw_ptr(&self) -> *const () {
        self.raw.0.data.data_ptr()
    }

    /// Get the id of the generational box.
    pub fn id(&self) -> GenerationalBoxId {
        GenerationalBoxId {
            data_ptr: self.raw.0.data.data_ptr(),
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: self.generation,
        }
    }

    /// Try to read the value. Returns None if the value is no longer valid.
    #[track_caller]
    pub fn try_read(&self) -> Result<S::Ref<T>, BorrowError> {
        if !self.validate() {
            return Err(BorrowError::Dropped(ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                created_at: self.created_at,
            }));
        }
        let result = self.raw.0.data.try_read(
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            GenerationalRefBorrowInfo {
                borrowed_at: std::panic::Location::caller(),
                borrowed_from: &self.raw.0.borrow,
                created_at: self.created_at,
            },
        );

        if result.is_ok() {
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            self.raw
                .0
                .borrow
                .borrowed_at
                .write()
                .push(std::panic::Location::caller());
        }

        result
    }

    /// Read the value. Panics if the value is no longer valid.
    #[track_caller]
    pub fn read(&self) -> S::Ref<T> {
        self.try_read().unwrap()
    }

    /// Try to write the value. Returns None if the value is no longer valid.
    #[track_caller]
    pub fn try_write(&self) -> Result<S::Mut<T>, BorrowMutError> {
        if !self.validate() {
            return Err(BorrowMutError::Dropped(ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                created_at: self.created_at,
            }));
        }
        let result = self.raw.0.data.try_write(
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            GenerationalRefMutBorrowInfo {
                borrowed_from: &self.raw.0.borrow,
                created_at: self.created_at,
            },
        );

        if result.is_ok() {
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            {
                *self.raw.0.borrow.borrowed_mut_at.write() = Some(std::panic::Location::caller());
            }
        }

        result
    }

    /// Write the value. Panics if the value is no longer valid.
    #[track_caller]
    pub fn write(&self) -> S::Mut<T> {
        self.try_write().unwrap()
    }

    /// Set the value. Panics if the value is no longer valid.
    pub fn set(&self, value: T) {
        self.validate().then(|| {
            self.raw.0.data.set(value);
        });
    }

    /// Returns true if the pointer is equal to the other pointer.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        {
            self.raw.0.data.data_ptr() == other.raw.0.data.data_ptr()
                && self.generation == other.generation
        }
        #[cfg(not(any(debug_assertions, feature = "check_generation")))]
        {
            self.raw.0.data.data_ptr() == other.raw.0.data.data_ptr()
        }
    }

    /// Take the value out of the generational box and invalidate the generational box. This will return the value if the value was taken.
    pub fn take(&self) -> Option<T> {
        if self.validate() {
            Storage::take(&self.raw.0.data)
        } else {
            None
        }
    }
}

impl<T, S: 'static> Copy for GenerationalBox<T, S> {}

impl<T, S> Clone for GenerationalBox<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A trait for a storage backing type. (RefCell, RwLock, etc.)
pub trait Storage<Data = ()>: AnyStorage + 'static {
    /// Try to read the value. Returns None if the value is no longer valid.
    fn try_read(
        &'static self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))] at: GenerationalRefBorrowInfo,
    ) -> Result<Self::Ref<Data>, BorrowError>;

    /// Try to write the value. Returns None if the value is no longer valid.
    fn try_write(
        &'static self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))] at: GenerationalRefMutBorrowInfo,
    ) -> Result<Self::Mut<Data>, BorrowMutError>;

    /// Set the value
    fn set(&'static self, value: Data);

    /// Take the value out of the storage. This will return the value if the value was taken.
    fn take(&'static self) -> Option<Data>;
}

/// A trait for any storage backing type.
pub trait AnyStorage: Default {
    /// The reference this storage type returns.
    type Ref<T: ?Sized + 'static>: Deref<Target = T> + 'static;
    /// The mutable reference this storage type returns.
    type Mut<T: ?Sized + 'static>: DerefMut<Target = T> + 'static;

    /// Try to map the mutable ref.
    fn try_map_mut<T: ?Sized, U: ?Sized + 'static>(
        mut_ref: Self::Mut<T>,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Self::Mut<U>>;

    /// Map the mutable ref.
    fn map_mut<T: ?Sized, U: ?Sized + 'static>(
        mut_ref: Self::Mut<T>,
        f: impl FnOnce(&mut T) -> &mut U,
    ) -> Self::Mut<U> {
        Self::try_map_mut(mut_ref, |v| Some(f(v))).unwrap()
    }

    /// Try to map the ref.
    fn try_map<T: ?Sized, U: ?Sized + 'static>(
        ref_: Self::Ref<T>,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<Self::Ref<U>>;

    /// Map the ref.
    fn map<T: ?Sized, U: ?Sized + 'static>(
        ref_: Self::Ref<T>,
        f: impl FnOnce(&T) -> &U,
    ) -> Self::Ref<U> {
        Self::try_map(ref_, |v| Some(f(v))).unwrap()
    }

    /// Get the data pointer. No guarantees are made about the data pointer. It should only be used for debugging.
    fn data_ptr(&self) -> *const ();

    /// Drop the value from the storage. This will return true if the value was taken.
    fn manually_drop(&self) -> bool;

    /// Recycle a memory location. This will drop the memory location and return it to the runtime.
    fn recycle(location: &MemoryLocation<Self>);

    /// Claim a new memory location. This will either create a new memory location or recycle an old one.
    fn claim() -> MemoryLocation<Self>;

    /// Create a new owner. The owner will be responsible for dropping all of the generational boxes that it creates.
    fn owner() -> Owner<Self> {
        Owner(Arc::new(Mutex::new(OwnerInner {
            owned: Default::default(),
        })))
    }
}

/// A dynamic memory location that can be used in a generational box.
pub struct MemoryLocation<S: 'static = UnsyncStorage>(&'static MemoryLocationInner<S>);

impl<S: 'static> Clone for MemoryLocation<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: 'static> Copy for MemoryLocation<S> {}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
#[derive(Debug, Default)]
struct MemoryLocationBorrowInfo {
    pub(crate) borrowed_at: parking_lot::RwLock<Vec<&'static std::panic::Location<'static>>>,
    pub(crate) borrowed_mut_at: parking_lot::RwLock<Option<&'static std::panic::Location<'static>>>,
}

#[cfg(any(debug_assertions, feature = "debug_ownership"))]
impl MemoryLocationBorrowInfo {
    fn borrow_mut_error(&self) -> BorrowMutError {
        if let Some(borrowed_mut_at) = self.borrowed_mut_at.read().as_ref() {
            BorrowMutError::AlreadyBorrowedMut(crate::error::AlreadyBorrowedMutError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrowed_mut_at,
            })
        } else {
            BorrowMutError::AlreadyBorrowed(crate::error::AlreadyBorrowedError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrowed_at: self.borrowed_at.read().clone(),
            })
        }
    }

    fn borrow_error(&self) -> BorrowError {
        BorrowError::AlreadyBorrowedMut(crate::error::AlreadyBorrowedMutError {
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            borrowed_mut_at: self.borrowed_mut_at.read().unwrap(),
        })
    }
}

struct MemoryLocationInner<S = UnsyncStorage> {
    data: S,

    #[cfg(any(debug_assertions, feature = "check_generation"))]
    generation: std::sync::atomic::AtomicU32,

    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrow: MemoryLocationBorrowInfo,
}

impl<S> MemoryLocation<S> {
    #[allow(unused)]
    fn drop(&self)
    where
        S: AnyStorage,
    {
        let old = self.0.data.manually_drop();
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        if old {
            let new_generation = self.0.generation.load(std::sync::atomic::Ordering::Relaxed) + 1;
            self.0
                .generation
                .store(new_generation, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn replace_with_caller<T: 'static>(
        &mut self,
        value: T,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        self.0.data.set(value);
        GenerationalBox {
            raw: *self,
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: self.0.generation.load(std::sync::atomic::Ordering::Relaxed),
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            created_at: caller,
            _marker: PhantomData,
        }
    }
}

struct OwnerInner<S: AnyStorage + 'static> {
    owned: Vec<MemoryLocation<S>>,
}

impl<S: AnyStorage> Drop for OwnerInner<S> {
    fn drop(&mut self) {
        for location in &self.owned {
            S::recycle(location)
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
        self.insert_with_caller(
            value,
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            std::panic::Location::caller(),
        )
    }

    /// Insert a value into the store with a specific location blamed for creating the value. The value will be dropped when the owner is dropped.
    pub fn insert_with_caller<T: 'static>(
        &self,
        value: T,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        let mut location = S::claim();
        let key = location.replace_with_caller(
            value,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            caller,
        );
        self.0.lock().owned.push(location);
        key
    }

    /// Creates an invalid handle. This is useful for creating a handle that will be filled in later. If you use this before the value is filled in, you will get may get a panic or an out of date value.
    pub fn invalid<T: 'static>(&self) -> GenerationalBox<T, S> {
        let location = S::claim();
        let generational_box = GenerationalBox {
            raw: location,
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: location
                .0
                .generation
                .load(std::sync::atomic::Ordering::Relaxed),
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            created_at: std::panic::Location::caller(),
            _marker: PhantomData,
        };
        self.0.lock().owned.push(location);
        generational_box
    }
}

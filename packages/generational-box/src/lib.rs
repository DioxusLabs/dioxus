#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use parking_lot::Mutex;
use std::sync::atomic::AtomicU32;
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

/// # Example
///
/// ```compile_fail
/// let data = String::from("hello world");
/// let owner = UnsyncStorage::owner();
/// let key = owner.insert(&data);
/// drop(data);
/// assert_eq!(*key.read(), "hello world");
/// ```
#[allow(unused)]
fn compile_fail() {}

#[test]
fn reused() {
    let first_ptr;
    {
        let owner = UnsyncStorage::owner();
        first_ptr = owner.insert(1).raw.0.data.data_ptr();
        drop(owner);
    }
    {
        let owner = UnsyncStorage::owner();
        let second_ptr = owner.insert(1234).raw.0.data.data_ptr();
        assert_eq!(first_ptr, second_ptr);
        drop(owner);
    }
}

#[test]
fn leaking_is_ok() {
    let data = String::from("hello world");
    let key;
    {
        // create an owner
        let owner = UnsyncStorage::owner();
        // insert data into the store
        key = owner.insert(data);
        // don't drop the owner
        std::mem::forget(owner);
    }
    assert_eq!(
        key.try_read().as_deref().unwrap(),
        &"hello world".to_string()
    );
}

#[test]
fn drops() {
    let data = String::from("hello world");
    let key;
    {
        // create an owner
        let owner = UnsyncStorage::owner();
        // insert data into the store
        key = owner.insert(data);
        // drop the owner
    }
    assert!(key.try_read().is_err());
}

#[test]
fn works() {
    let owner = UnsyncStorage::owner();
    let key = owner.insert(1);

    assert_eq!(*key.read(), 1);
}

#[test]
fn insert_while_reading() {
    let owner = UnsyncStorage::owner();
    let key;
    {
        let data: String = "hello world".to_string();
        key = owner.insert(data);
    }
    let value = key.read();
    owner.insert(&1);
    assert_eq!(*value, "hello world");
}

#[test]
#[should_panic]
fn panics() {
    let owner = UnsyncStorage::owner();
    let key = owner.insert(1);
    drop(owner);

    assert_eq!(*key.read(), 1);
}

#[test]
fn fuzz() {
    fn maybe_owner_scope(
        valid_keys: &mut Vec<GenerationalBox<String>>,
        invalid_keys: &mut Vec<GenerationalBox<String>>,
        path: &mut Vec<u8>,
    ) {
        let branch_cutoff = 5;
        let children = if path.len() < branch_cutoff {
            rand::random::<u8>() % 4
        } else {
            rand::random::<u8>() % 2
        };

        for i in 0..children {
            let owner = UnsyncStorage::owner();
            let key = owner.insert(format!("hello world {path:?}"));
            valid_keys.push(key);
            path.push(i);
            // read all keys
            println!("{:?}", path);
            for key in valid_keys.iter() {
                let value = key.read();
                println!("{:?}", &*value);
                assert!(value.starts_with("hello world"));
            }
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            for key in invalid_keys.iter() {
                assert!(!key.validate());
            }
            maybe_owner_scope(valid_keys, invalid_keys, path);
            invalid_keys.push(valid_keys.pop().unwrap());
            path.pop();
        }
    }

    for _ in 0..10 {
        maybe_owner_scope(&mut Vec::new(), &mut Vec::new(), &mut Vec::new());
    }
}

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
        f.write_fmt(format_args!("{:?}", self.raw.0.data.as_ptr()))?;
        Ok(())
    }
}

impl<T: 'static, S: Storage<T>> GenerationalBox<T, S> {
    #[inline(always)]
    fn validate(&self) -> bool {
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
    pub fn try_read(&self) -> Result<S::Ref, BorrowError> {
        if !self.validate() {
            return Err(BorrowError::Dropped(ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                created_at: self.created_at,
            }));
        }
        let result = self.raw.0.data.try_read(
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            self.created_at,
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            GenerationalRefBorrowInfo {
                borrowed_at: std::panic::Location::caller(),
                borrowed_from: &self.raw.0.borrow,
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
    pub fn read(&self) -> S::Ref {
        self.try_read().unwrap()
    }

    /// Try to write the value. Returns None if the value is no longer valid.
    #[track_caller]
    pub fn try_write(&self) -> Result<S::Mut, BorrowMutError> {
        if !self.validate() {
            return Err(BorrowMutError::Dropped(ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                created_at: self.created_at,
            }));
        }
        let result = self.raw.0.data.try_write(
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            self.created_at,
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            GenerationalRefMutBorrowInfo {
                borrowed_from: &self.raw.0.borrow,
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
    pub fn write(&self) -> S::Mut {
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
            self.raw.data.as_ptr() == other.raw.data.as_ptr()
        }
    }
}

impl<T, S: 'static> Copy for GenerationalBox<T, S> {}

impl<T, S> Clone for GenerationalBox<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A trait for types that can be mapped.
pub trait Mappable<T: ?Sized>: Deref<Target = T> {
    /// The type after the mapping.
    type Mapped<U: ?Sized + 'static>: Mappable<U> + Deref<Target = U>;

    /// Map the value.
    fn map<U: ?Sized + 'static>(_self: Self, f: impl FnOnce(&T) -> &U) -> Self::Mapped<U>;

    /// Try to map the value.
    fn try_map<U: ?Sized + 'static>(
        _self: Self,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<Self::Mapped<U>>;
}

/// A trait for types that can be mapped mutably.
pub trait MappableMut<T: ?Sized>: DerefMut<Target = T> {
    /// The type after the mapping.
    type Mapped<U: ?Sized + 'static>: MappableMut<U> + DerefMut<Target = U>;

    /// Map the value.
    fn map<U: ?Sized + 'static>(_self: Self, f: impl FnOnce(&mut T) -> &mut U) -> Self::Mapped<U>;

    /// Try to map the value.
    fn try_map<U: ?Sized + 'static>(
        _self: Self,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Self::Mapped<U>>;
}

/// A trait for a storage backing type. (RefCell, RwLock, etc.)
pub trait Storage<Data>: AnyStorage + 'static {
    /// The reference this storage type returns.
    type Ref: Mappable<Data> + Deref<Target = Data>;
    /// The mutable reference this storage type returns.
    type Mut: MappableMut<Data> + DerefMut<Target = Data>;

    /// Try to read the value. Returns None if the value is no longer valid.
    fn try_read(
        &'static self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        created_at: &'static std::panic::Location<'static>,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))] at: GenerationalRefBorrowInfo,
    ) -> Result<Self::Ref, BorrowError>;

    /// Try to write the value. Returns None if the value is no longer valid.
    fn try_write(
        &'static self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        created_at: &'static std::panic::Location<'static>,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))] at: GenerationalRefMutBorrowInfo,
    ) -> Result<Self::Mut, BorrowMutError>;

    /// Set the value
    fn set(&'static self, value: Data);
}

/// A trait for any storage backing type.
pub trait AnyStorage: Default {
    /// Get the data pointer. No guarantees are made about the data pointer. It should only be used for debugging.
    fn data_ptr(&self) -> *const ();

    /// Take the value out of the storage. This will return true if the value was taken.
    fn take(&self) -> bool;

    /// Recycle a memory location. This will drop the memory location and return it to the runtime.
    fn recycle(location: &MemoryLocation<Self>);

    /// Claim a new memory location. This will either create a new memory location or recycle an old one.
    fn claim() -> MemoryLocation<Self>;

    /// Create a new owner. The owner will be responsible for dropping all of the generational boxes that it creates.
    fn owner() -> Owner<Self> {
        Owner {
            owned: Default::default(),
            phantom: PhantomData,
        }
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
    generation: AtomicU32,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrow: MemoryLocationBorrowInfo,
}

impl<S> MemoryLocation<S> {
    #[allow(unused)]
    fn drop(&self)
    where
        S: AnyStorage,
    {
        let old = self.0.data.take();
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

/// Owner: Handles dropping generational boxes. The owner acts like a runtime lifetime guard. Any states that you create with an owner will be dropped when that owner is dropped.
pub struct Owner<S: AnyStorage + 'static = UnsyncStorage> {
    owned: Arc<Mutex<Vec<MemoryLocation<S>>>>,
    phantom: PhantomData<S>,
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
        self.owned.lock().push(location);
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
        self.owned.lock().push(location);
        generational_box
    }
}

impl<S: AnyStorage> Drop for Owner<S> {
    fn drop(&mut self) {
        for location in self.owned.lock().iter() {
            S::recycle(location)
        }
    }
}

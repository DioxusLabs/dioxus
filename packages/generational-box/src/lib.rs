#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::any::Any;
use std::error::Error;
use std::fmt::Display;
use std::sync::atomic::AtomicU32;
use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, OnceLock},
};

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
        first_ptr = owner.insert(1).raw.data.data_ptr();
        drop(owner);
    }
    {
        let owner = UnsyncStorage::owner();
        let second_ptr = owner.insert(1234).raw.data.data_ptr();
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
        self.raw.0.data.try_read().ok_or_else(|| {
            BorrowError::Dropped(ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                created_at: self.created_at,
            })
        })
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
        self.raw.0.data.try_write().ok_or_else(|| {
            BorrowMutError::Dropped(ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                created_at: self.created_at,
            })
        })
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

impl<T, S: 'static + Copy> Copy for GenerationalBox<T, S> {}

impl<T, S: Copy> Clone for GenerationalBox<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A unsync storage. This is the default storage type.
pub struct UnsyncStorage(RefCell<Option<Box<dyn std::any::Any>>>);

impl Default for UnsyncStorage {
    fn default() -> Self {
        Self(RefCell::new(None))
    }
}

/// A thread safe storage. This is slower than the unsync storage, but allows you to share the value between threads.
#[derive(Clone, Copy)]
pub struct SyncStorage(&'static RwLock<Option<Box<dyn std::any::Any + Send + Sync>>>);

impl Default for SyncStorage {
    fn default() -> Self {
        Self(Box::leak(Box::new(RwLock::new(None))))
    }
}

/// A trait for types that can be mapped.
pub trait Mappable<T>: Deref<Target = T> {
    /// The type after the mapping.
    type Mapped<U: 'static>: Mappable<U> + Deref<Target = U>;

    /// Map the value.
    fn map<U: 'static>(_self: Self, f: impl FnOnce(&T) -> &U) -> Self::Mapped<U>;

    /// Try to map the value.
    fn try_map<U: 'static>(
        _self: Self,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<Self::Mapped<U>>;
}

impl<T> Mappable<T> for Ref<'static, T> {
    type Mapped<U: 'static> = Ref<'static, U>;

    fn map<U: 'static>(_self: Self, f: impl FnOnce(&T) -> &U) -> Self::Mapped<U> {
        Ref::map(_self, f)
    }

    fn try_map<U: 'static>(
        _self: Self,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<Self::Mapped<U>> {
        Ref::filter_map(_self, f).ok()
    }
}

impl<T> Mappable<T> for MappedRwLockReadGuard<'static, T> {
    type Mapped<U: 'static> = MappedRwLockReadGuard<'static, U>;

    fn map<U: 'static>(_self: Self, f: impl FnOnce(&T) -> &U) -> Self::Mapped<U> {
        MappedRwLockReadGuard::map(_self, f)
    }

    fn try_map<U: 'static>(
        _self: Self,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<Self::Mapped<U>> {
        MappedRwLockReadGuard::try_map(_self, f).ok()
    }
}

/// A trait for types that can be mapped mutably.
pub trait MappableMut<T>: DerefMut<Target = T> {
    /// The type after the mapping.
    type Mapped<U: 'static>: MappableMut<U> + DerefMut<Target = U>;

    /// Map the value.
    fn map<U: 'static>(_self: Self, f: impl FnOnce(&mut T) -> &mut U) -> Self::Mapped<U>;

    /// Try to map the value.
    fn try_map<U: 'static>(
        _self: Self,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Self::Mapped<U>>;
}

impl<T> MappableMut<T> for RefMut<'static, T> {
    type Mapped<U: 'static> = RefMut<'static, U>;

    fn map<U: 'static>(_self: Self, f: impl FnOnce(&mut T) -> &mut U) -> Self::Mapped<U> {
        RefMut::map(_self, f)
    }

    fn try_map<U: 'static>(
        _self: Self,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Self::Mapped<U>> {
        RefMut::filter_map(_self, f).ok()
    }
}

impl<T> MappableMut<T> for MappedRwLockWriteGuard<'static, T> {
    type Mapped<U: 'static> = MappedRwLockWriteGuard<'static, U>;

    fn map<U: 'static>(_self: Self, f: impl FnOnce(&mut T) -> &mut U) -> Self::Mapped<U> {
        MappedRwLockWriteGuard::map(_self, f)
    }

    fn try_map<U: 'static>(
        _self: Self,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Self::Mapped<U>> {
        MappedRwLockWriteGuard::try_map(_self, f).ok()
    }
}

/// A trait for a storage backing type. (RefCell, RwLock, etc.)
pub trait Storage<Data>: AnyStorage + 'static {
    /// The reference this storage type returns.
    type Ref: Mappable<Data> + Deref<Target = Data>;
    /// The mutable reference this storage type returns.
    type Mut: MappableMut<Data> + DerefMut<Target = Data>;

    /// Try to read the value. Returns None if the value is no longer valid.
    fn try_read(&self) -> Option<Self::Ref>;
    /// Read the value. Panics if the value is no longer valid.
    fn read(&self) -> Self::Ref {
        self.try_read()
            .expect("generational box has been invalidated or the type has changed")
    }
    /// Try to write the value. Returns None if the value is no longer valid.
    fn try_write(&self) -> Option<Self::Mut>;
    /// Write the value. Panics if the value is no longer valid.
    fn write(&self) -> Self::Mut {
        self.try_write()
            .expect("generational box has been invalidated or the type has changed")
    }

    /// Set the value. Panics if the value is no longer valid.
    fn set(&self, value: Data);
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

impl<T: 'static> Storage<T> for UnsyncStorage {
    type Ref = Ref<'static, T>;
    type Mut = RefMut<'static, T>;

    fn try_read(&self) -> Option<Self::Ref> {
        Ref::filter_map(self.0.borrow(), |any| any.as_ref()?.downcast_ref()).ok()
    }

    fn try_write(&self) -> Option<Self::Mut> {
        RefMut::filter_map(self.0.borrow_mut(), |any| any.as_mut()?.downcast_mut()).ok()
    }

    fn set(&self, value: T) {
        *self.0.borrow_mut() = Some(Box::new(value));
    }
}

thread_local! {
    static UNSYNC_RUNTIME: RefCell<Vec<MemoryLocation<UnsyncStorage>>> = RefCell::new(Vec::new());
}

impl AnyStorage for UnsyncStorage {
    fn data_ptr(&self) -> *const () {
        self.0.as_ptr() as *const ()
    }

    fn take(&self) -> bool {
        self.0.borrow_mut().take().is_some()
    }

    fn claim() -> MemoryLocation<Self> {
        UNSYNC_RUNTIME.with(|runtime| {
            if let Some(location) = runtime.borrow_mut().pop() {
                location
            } else {
                let data: &'static MemoryLocationInner =
                    &*Box::leak(Box::new(MemoryLocationInner {
                        data: Self::default(),
                        #[cfg(any(debug_assertions, feature = "check_generation"))]
                        generation: 0.into(),
                        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                        borrowed_at: Default::default(),
                        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                        borrowed_mut_at: Default::default(),
                    }));
                MemoryLocation(data)
            }
        })
    }

    fn recycle(location: &MemoryLocation<Self>) {
        location.drop();
        UNSYNC_RUNTIME.with(|runtime| runtime.borrow_mut().push(*location));
    }
}

impl<T: Sync + Send + 'static> Storage<T> for SyncStorage {
    type Ref = MappedRwLockReadGuard<'static, T>;
    type Mut = MappedRwLockWriteGuard<'static, T>;

    fn try_read(&self) -> Option<Self::Ref> {
        RwLockReadGuard::try_map(self.0.read(), |any| any.as_ref()?.downcast_ref()).ok()
    }

    fn try_write(&self) -> Option<Self::Mut> {
        RwLockWriteGuard::try_map(self.0.write(), |any| any.as_mut()?.downcast_mut()).ok()
    }

    fn set(&self, value: T) {
        *self.0.write() = Some(Box::new(value));
    }
}

static SYNC_RUNTIME: OnceLock<Arc<Mutex<Vec<MemoryLocation<SyncStorage>>>>> = OnceLock::new();

fn sync_runtime() -> &'static Arc<Mutex<Vec<MemoryLocation<SyncStorage>>>> {
    SYNC_RUNTIME.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

impl AnyStorage for SyncStorage {
    fn data_ptr(&self) -> *const () {
        self.0.data_ptr() as *const ()
    }

    fn take(&self) -> bool {
        self.0.write().take().is_some()
    }

    fn claim() -> MemoryLocation<Self> {
        sync_runtime().lock().pop().unwrap_or_else(|| {
            let data: &'static MemoryLocationInner<Self> =
                &*Box::leak(Box::new(MemoryLocationInner {
                    data: Self::default(),
                    #[cfg(any(debug_assertions, feature = "check_generation"))]
                    generation: 0.into(),
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    borrowed_at: Default::default(),
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    borrowed_mut_at: Default::default(),
                }));
            MemoryLocation(data)
        })
    }

    fn recycle(location: &MemoryLocation<Self>) {
        location.drop();
        sync_runtime().lock().push(*location);
    }
}

#[derive(Clone, Copy)]
struct MemoryLocation<S: 'static = UnsyncStorage>(&'static MemoryLocationInner<S>);

struct MemoryLocationInner<S = UnsyncStorage> {
    data: S,
    #[cfg(any(debug_assertions, feature = "check_generation"))]
    generation: AtomicU32,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrowed_at: RwLock<Vec<&'static std::panic::Location<'static>>>,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrowed_mut_at: RwLock<Option<&'static std::panic::Location<'static>>>,
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

    #[track_caller]
    fn try_borrow<T: Any>(
        &self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        created_at: &'static std::panic::Location<'static>,
    ) -> Result<GenerationalRef<T, S>, BorrowError>
    where
        S: Storage<T>,
    {
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        self.0
            .borrowed_at
            .write()
            .push(std::panic::Location::caller());
        match self.0.data.try_read() {
            Some(borrow) => {
                match Ref::filter_map(borrow, |any| any.as_ref()?.downcast_ref::<T>()) {
                    Ok(reference) => Ok(GenerationalRef {
                        inner: reference,
                        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                        borrow: GenerationalRefBorrowInfo {
                            borrowed_at: std::panic::Location::caller(),
                            borrowed_from: self.0,
                        },
                    }),
                    Err(_) => Err(BorrowError::Dropped(ValueDroppedError {
                        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                        created_at,
                    })),
                }
            }
            None => Err(BorrowError::AlreadyBorrowedMut(AlreadyBorrowedMutError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrowed_mut_at: self.0.borrowed_mut_at.read().unwrap(),
            })),
        }
    }

    #[track_caller]
    fn try_borrow_mut<T: Any>(
        &self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        created_at: &'static std::panic::Location<'static>,
    ) -> Result<GenerationalRefMut<T>, BorrowMutError> {
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        {
            self.0.borrowed_mut_at.write().unwrap() = Some(std::panic::Location::caller());
        }
        match self.0.data.try_borrow_mut() {
            Ok(borrow_mut) => {
                match RefMut::filter_map(borrow_mut, |any| any.as_mut()?.downcast_mut::<T>()) {
                    Ok(reference) => Ok(GenerationalRefMut {
                        inner: reference,
                        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                        borrow: GenerationalRefMutBorrowInfo {
                            borrowed_from: self.0,
                        },
                    }),
                    Err(_) => Err(BorrowMutError::Dropped(ValueDroppedError {
                        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                        created_at,
                    })),
                }
            }
            Err(_) => Err(BorrowMutError::AlreadyBorrowed(AlreadyBorrowedError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrowed_at: self.0.borrowed_at.read().clone(),
            })),
        }
    }
}

#[derive(Debug, Clone)]
/// An error that can occur when trying to borrow a value.
pub enum BorrowError {
    /// The value was dropped.
    Dropped(ValueDroppedError),
    /// The value was already borrowed mutably.
    AlreadyBorrowedMut(AlreadyBorrowedMutError),
}

impl Display for BorrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowError::Dropped(error) => Display::fmt(error, f),
            BorrowError::AlreadyBorrowedMut(error) => Display::fmt(error, f),
        }
    }
}

impl Error for BorrowError {}

#[derive(Debug, Clone)]
/// An error that can occur when trying to borrow a value mutably.
pub enum BorrowMutError {
    /// The value was dropped.
    Dropped(ValueDroppedError),
    /// The value was already borrowed.
    AlreadyBorrowed(AlreadyBorrowedError),
    /// The value was already borrowed mutably.
    AlreadyBorrowedMut(AlreadyBorrowedMutError),
}

impl Display for BorrowMutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorrowMutError::Dropped(error) => Display::fmt(error, f),
            BorrowMutError::AlreadyBorrowedMut(error) => Display::fmt(error, f),
            BorrowMutError::AlreadyBorrowed(error) => Display::fmt(error, f),
        }
    }
}

impl Error for BorrowMutError {}

/// An error that can occur when trying to use a value that has been dropped.
#[derive(Debug, Copy, Clone)]
pub struct ValueDroppedError {
    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
    created_at: &'static std::panic::Location<'static>,
}

impl Display for ValueDroppedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to borrow because the value was dropped.")?;
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        f.write_fmt(format_args!("created_at: {}", self.created_at))?;
        Ok(())
    }
}

impl std::error::Error for ValueDroppedError {}

/// An error that can occur when trying to borrow a value that has already been borrowed mutably.
#[derive(Debug, Copy, Clone)]
pub struct AlreadyBorrowedMutError {
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrowed_mut_at: &'static std::panic::Location<'static>,
}

impl Display for AlreadyBorrowedMutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to borrow because the value was already borrowed mutably.")?;
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        f.write_fmt(format_args!("borrowed_mut_at: {}", self.borrowed_mut_at))?;
        Ok(())
    }
}

impl std::error::Error for AlreadyBorrowedMutError {}

/// An error that can occur when trying to borrow a value mutably that has already been borrowed immutably.
#[derive(Debug, Clone)]
pub struct AlreadyBorrowedError {
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrowed_at: Vec<&'static std::panic::Location<'static>>,
}

impl Display for AlreadyBorrowedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to borrow mutably because the value was already borrowed immutably.")?;
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        f.write_str("borrowed_at:")?;
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        for location in self.borrowed_at.iter() {
            f.write_fmt(format_args!("\t{}", location))?;
        }
        Ok(())
    }
}

impl std::error::Error for AlreadyBorrowedError {}

/// A reference to a value in a generational box.
pub struct GenerationalRef<T: 'static> {
    inner: Ref<'static, T>,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrow: GenerationalRefBorrowInfo,
}

impl<T: 'static> GenerationalRef<T> {
    /// Map one ref type to another.
    pub fn map<U, F>(orig: GenerationalRef<T>, f: F) -> GenerationalRef<U>
    where
        F: FnOnce(&T) -> &U,
    {
        GenerationalRef {
            inner: Ref::map(orig.inner, f),
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow: GenerationalRefBorrowInfo {
                borrowed_at: orig.borrow.borrowed_at,
                borrowed_from: orig.borrow.borrowed_from,
            },
        }
    }

    /// Filter one ref type to another.
    pub fn filter_map<U, F>(orig: GenerationalRef<T>, f: F) -> Option<GenerationalRef<U>>
    where
        F: FnOnce(&T) -> Option<&U>,
    {
        let Self {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
        } = orig;
        Ref::filter_map(inner, f).ok().map(|inner| GenerationalRef {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow: GenerationalRefBorrowInfo {
                borrowed_at: borrow.borrowed_at,
                borrowed_from: borrow.borrowed_from,
            },
        })
    }
}

impl<T: 'static> Deref for GenerationalRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
struct GenerationalRefBorrowInfo {
    borrowed_at: &'static std::panic::Location<'static>,
    borrowed_from: &'static MemoryLocationInner,
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
impl Drop for GenerationalRefBorrowInfo {
    fn drop(&mut self) {
        self.borrowed_from
            .borrowed_at
            .borrow_mut()
            .retain(|location| std::ptr::eq(*location, self.borrowed_at as *const _));
    }
}

/// A mutable reference to a value in a generational box.
pub struct GenerationalRefMut<T: 'static> {
    inner: RefMut<'static, T>,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrow: GenerationalRefMutBorrowInfo,
}

impl<T: 'static> GenerationalRefMut<T> {
    /// Map one ref type to another.
    pub fn map<U, F>(orig: GenerationalRefMut<T>, f: F) -> GenerationalRefMut<U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        GenerationalRefMut {
            inner: RefMut::map(orig.inner, f),
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow: orig.borrow,
        }
    }

    /// Filter one ref type to another.
    pub fn filter_map<U, F>(orig: GenerationalRefMut<T>, f: F) -> Option<GenerationalRefMut<U>>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        let Self {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
        } = orig;
        RefMut::filter_map(inner, f)
            .ok()
            .map(|inner| GenerationalRefMut {
                inner,
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrow,
            })
    }
}

impl<T: 'static> Deref for GenerationalRefMut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: 'static> DerefMut for GenerationalRefMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
struct GenerationalRefMutBorrowInfo {
    borrowed_from: &'static MemoryLocationInner,
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
impl Drop for GenerationalRefMutBorrowInfo {
    fn drop(&mut self) {
        self.borrowed_from.borrowed_mut_at.take();
    }
}

/// Owner: Handles dropping generational boxes. The owner acts like a runtime lifetime guard. Any states that you create with an owner will be dropped when that owner is dropped.
pub struct Owner<S: AnyStorage + 'static = UnsyncStorage> {
    owned: Arc<Mutex<Vec<MemoryLocation<S>>>>,
    phantom: PhantomData<S>,
}

impl<S: AnyStorage + Copy> Owner<S> {
    /// Insert a value into the store. The value will be dropped when the owner is dropped.
    #[track_caller]
    pub fn insert<T: 'static>(&self, value: T) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        let mut location = S::claim();
        let key = location.replace_with_caller(
            value,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            std::panic::Location::caller(),
        );
        self.owned.lock().push(location);
        key
    }

    /// Insert a value into the store with a specific location blamed for creating the value. The value will be dropped when the owner is dropped.
    pub fn insert_with_caller<T: 'static>(
        &self,
        value: T,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalBox<T> {
        let mut location = self.store.claim();
        let key = location.replace_with_caller(
            value,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            caller,
        );
        self.owned.borrow_mut().push(location);
        key
    }

    /// Creates an invalid handle. This is useful for creating a handle that will be filled in later. If you use this before the value is filled in, you will get may get a panic or an out of date value.
    pub fn invalid<T: 'static>(&self) -> GenerationalBox<T, S> {
        let location = S::claim();
        GenerationalBox {
            raw: location,
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: location
                .0
                .generation
                .load(std::sync::atomic::Ordering::Relaxed),
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            created_at: std::panic::Location::caller(),
            _marker: PhantomData,
        }
    }
}

impl<S: AnyStorage> Drop for Owner<S> {
    fn drop(&mut self) {
        for location in self.owned.lock().iter() {
            S::recycle(location)
        }
    }
}

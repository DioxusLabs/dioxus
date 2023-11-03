#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::{atomic::AtomicU32, Arc, OnceLock},
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
    assert_eq!(key.try_read().as_deref(), Some(&"hello world".to_string()));
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
    assert!(key.try_read().is_none());
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
                println!("{:?}", value);
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

/// The core Copy state type. The generational box will be dropped when the [Owner] is dropped.
pub struct GenerationalBox<T, S = UnsyncStorage> {
    raw: MemoryLocation<S>,
    #[cfg(any(debug_assertions, feature = "check_generation"))]
    generation: u32,
    _marker: PhantomData<T>,
}

impl<T: 'static, S: AnyStorage> Debug for GenerationalBox<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        f.write_fmt(format_args!(
            "{:?}@{:?}",
            self.raw.data.data_ptr(),
            self.generation
        ))?;
        #[cfg(not(any(debug_assertions, feature = "check_generation")))]
        f.write_fmt(format_args!("{:?}", self.raw.data.as_ptr()))?;
        Ok(())
    }
}

impl<T: 'static, S: Storage<T>> GenerationalBox<T, S> {
    #[inline(always)]
    fn validate(&self) -> bool {
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        {
            self.raw
                .generation
                .load(std::sync::atomic::Ordering::Relaxed)
                == self.generation
        }
        #[cfg(not(any(debug_assertions, feature = "check_generation")))]
        {
            true
        }
    }

    /// Try to read the value. Returns None if the value is no longer valid.
    pub fn try_read(&self) -> Option<S::Ref> {
        self.validate().then(|| self.raw.data.try_read()).flatten()
    }

    /// Read the value. Panics if the value is no longer valid.
    pub fn read(&self) -> S::Ref {
        self.try_read().unwrap()
    }

    /// Try to write the value. Returns None if the value is no longer valid.
    pub fn try_write(&self) -> Option<S::Mut> where {
        self.validate().then(|| self.raw.data.try_write()).flatten()
    }

    /// Write the value. Panics if the value is no longer valid.
    pub fn write(&self) -> S::Mut {
        self.try_write().unwrap()
    }

    /// Set the value. Panics if the value is no longer valid.
    pub fn set(&self, value: T) {
        self.validate().then(|| {
            self.raw.data.set(value);
        });
    }

    /// Returns true if the pointer is equal to the other pointer.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        {
            self.raw.data.data_ptr() == other.raw.data.data_ptr()
                && self.generation == other.generation
        }
        #[cfg(not(any(debug_assertions, feature = "check_generation")))]
        {
            self.raw.data.as_ptr() == other.raw.data.as_ptr()
        }
    }
}

impl<T, S: Copy> Copy for GenerationalBox<T, S> {}

impl<T, S: Copy> Clone for GenerationalBox<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Clone, Copy)]
pub struct UnsyncStorage(&'static RefCell<Option<Box<dyn std::any::Any>>>);

impl Default for UnsyncStorage {
    fn default() -> Self {
        Self(Box::leak(Box::new(RefCell::new(None))))
    }
}

#[derive(Clone, Copy)]
pub struct SyncStorage(&'static RwLock<Option<Box<dyn std::any::Any + Send + Sync>>>);

impl Default for SyncStorage {
    fn default() -> Self {
        Self(Box::leak(Box::new(RwLock::new(None))))
    }
}

pub trait Mappable<T>: Deref<Target = T> {
    type Mapped<U: 'static>: Mappable<U> + Deref<Target = U>;

    fn map<U: 'static>(_self: Self, f: fn(&T) -> &U) -> Self::Mapped<U>;

    fn try_map<U: 'static>(_self: Self, f: fn(&T) -> Option<&U>) -> Option<Self::Mapped<U>>;
}

impl<T> Mappable<T> for Ref<'static, T> {
    type Mapped<U: 'static> = Ref<'static, U>;

    fn map<U: 'static>(_self: Self, f: fn(&T) -> &U) -> Self::Mapped<U> {
        Ref::map(_self, f)
    }

    fn try_map<U: 'static>(_self: Self, f: fn(&T) -> Option<&U>) -> Option<Self::Mapped<U>> {
        Ref::try_map(_self, f)
    }
}

impl<T> Mappable<T> for MappedRwLockReadGuard<'static, T> {
    type Mapped<U: 'static> = MappedRwLockReadGuard<'static, U>;

    fn map<U: 'static>(_self: Self, f: fn(&T) -> &U) -> Self::Mapped<U> {
        MappedRwLockReadGuard::map(_self, f)
    }

    fn try_map<U: 'static>(_self: Self, f: fn(&T) -> Option<&U>) -> Option<Self::Mapped<U>> {
        MappedRwLockReadGuard::try_map(_self, f).ok()
    }
}

pub trait MappableMut<T>: DerefMut<Target = T> {
    type Mapped<U: 'static>: MappableMut<U> + DerefMut<Target = U>;

    fn map<U: 'static>(_self: Self, f: impl FnOnce(&mut T) -> &mut U) -> Self::Mapped<U>;

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
        RefMut::try_map(_self, f)
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

pub trait Storage<Data>: Copy + AnyStorage {
    type Ref: Mappable<Data> + Deref<Target = Data>;
    type Mut: MappableMut<Data> + DerefMut<Target = Data>;

    fn try_read(&self) -> Option<Self::Ref>;
    fn read(&self) -> Self::Ref {
        self.try_read()
            .expect("generational box has been invalidated or the type has changed")
    }
    fn try_write(&self) -> Option<Self::Mut>;
    fn write(&self) -> Self::Mut {
        self.try_write()
            .expect("generational box has been invalidated or the type has changed")
    }

    fn set(&self, value: Data);
}

pub trait AnyStorage: Default {
    fn data_ptr(&self) -> *const ();

    fn take(&self) -> bool;

    fn recycle(location: &MemoryLocation<Self>);
    // {
    //     location.drop();
    //     self.recycled.lock().push(location);
    // }

    fn claim() -> MemoryLocation<Self>;
    // where
    //     S: Default,
    // {
    //     if let Some(location) = self.recycled.lock().pop() {
    //         location
    //     } else {
    //         MemoryLocation {
    //             data: Default::default(),
    //             #[cfg(any(debug_assertions, feature = "check_generation"))]
    //             generation: Box::leak(Box::new(Default::default())),
    //         }
    //     }
    // }

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
                MemoryLocation {
                    data: UnsyncStorage(Box::leak(Box::new(RefCell::new(None)))),
                    #[cfg(any(debug_assertions, feature = "check_generation"))]
                    generation: Box::leak(Box::new(Default::default())),
                }
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
        MemoryLocation {
            data: SyncStorage(Box::leak(Box::new(RwLock::new(None)))),
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: Box::leak(Box::new(Default::default())),
        }
    }

    fn recycle(location: &MemoryLocation<Self>) {
        location.drop();
        sync_runtime().lock().push(*location);
    }
}

#[derive(Clone, Copy)]
struct MemoryLocation<S = UnsyncStorage> {
    data: S,
    #[cfg(any(debug_assertions, feature = "check_generation"))]
    generation: &'static AtomicU32,
}

impl<S> MemoryLocation<S> {
    #[allow(unused)]
    fn drop(&self)
    where
        S: AnyStorage,
    {
        let old = self.data.take();
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        if old {
            let new_generation = self.generation.load(std::sync::atomic::Ordering::Relaxed) + 1;
            self.generation
                .store(new_generation, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn replace<T: 'static>(&mut self, value: T) -> GenerationalBox<T, S>
    where
        S: Storage<T> + Copy,
    {
        self.data.set(value);
        GenerationalBox {
            raw: *self,
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: self.generation.load(std::sync::atomic::Ordering::Relaxed),
            _marker: PhantomData,
        }
    }
}

/// Owner: Handles dropping generational boxes. The owner acts like a runtime lifetime guard. Any states that you create with an owner will be dropped when that owner is dropped.
pub struct Owner<S: AnyStorage = UnsyncStorage> {
    owned: Arc<Mutex<Vec<MemoryLocation<S>>>>,
    phantom: PhantomData<S>,
}

impl<S: AnyStorage + Copy> Owner<S> {
    /// Insert a value into the store. The value will be dropped when the owner is dropped.
    pub fn insert<T: 'static>(&self, value: T) -> GenerationalBox<T, S>
    where
        S: Storage<T>,
    {
        let mut location = S::claim();
        let key = location.replace(value);
        self.owned.lock().push(location);
        key
    }

    /// Creates an invalid handle. This is useful for creating a handle that will be filled in later. If you use this before the value is filled in, you will get may get a panic or an out of date value.
    pub fn invalid<T: 'static>(&self) -> GenerationalBox<T, S> {
        let location = S::claim();
        GenerationalBox {
            raw: location,
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: location
                .generation
                .load(std::sync::atomic::Ordering::Relaxed),
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

#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::{
    cell::RefCell,
    fmt::Debug,
    marker::PhantomData,
    rc::Rc,
    sync::{atomic::AtomicU32, Arc},
};

mod testing;

/// # Example
///
/// ```compile_fail
/// let data = String::from("hello world");
/// let store = Store::default();
/// let owner = store.owner();
/// let key = owner.insert(&data);
/// drop(data);
/// assert_eq!(*key.read(), "hello world");
/// ```
#[allow(unused)]
fn compile_fail() {}

#[test]
fn reused() {
    let store = Store::default();
    let first_ptr;
    {
        let owner = store.owner();
        first_ptr = owner.insert(1).raw.data.data_ptr();
        drop(owner);
    }
    {
        let owner = store.owner();
        let second_ptr = owner.insert(1234).raw.data.data_ptr();
        assert_eq!(first_ptr, second_ptr);
        drop(owner);
    }
}

#[test]
fn leaking_is_ok() {
    let data = String::from("hello world");
    let store = Store::default();
    let key;
    {
        // create an owner
        let owner = store.owner();
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
    let store = Store::default();
    let key;
    {
        // create an owner
        let owner = store.owner();
        // insert data into the store
        key = owner.insert(data);
        // drop the owner
    }
    assert!(key.try_read().is_none());
}

#[test]
fn works() {
    let store = Store::default();
    let owner = store.owner();
    let key = owner.insert(1);

    assert_eq!(*key.read(), 1);
}

#[test]
fn insert_while_reading() {
    let store = Store::default();
    let owner = store.owner();
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
    let store = Store::default();
    let owner = store.owner();
    let key = owner.insert(1);
    drop(owner);

    assert_eq!(*key.read(), 1);
}

#[test]
fn fuzz() {
    fn maybe_owner_scope(
        store: &Store,
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
            let owner = store.owner();
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
            maybe_owner_scope(store, valid_keys, invalid_keys, path);
            invalid_keys.push(valid_keys.pop().unwrap());
            path.pop();
        }
    }

    for _ in 0..10 {
        let store = Store::default();
        maybe_owner_scope(&store, &mut Vec::new(), &mut Vec::new(), &mut Vec::new());
    }
}

/// The core Copy state type. The generational box will be dropped when the [Owner] is dropped.
pub struct GenerationalBox<T> {
    raw: MemoryLocation,
    #[cfg(any(debug_assertions, feature = "check_generation"))]
    generation: u32,
    _marker: PhantomData<T>,
}

impl<T: 'static> Debug for GenerationalBox<T> {
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

impl<T: 'static> GenerationalBox<T> {
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
    pub fn try_read(&self) -> Option<MappedRwLockReadGuard<'static, T>> {
        self.validate()
            .then(|| {
                RwLockReadGuard::try_map(self.raw.data.read(), |any| {
                    any.as_ref()?.downcast_ref::<T>()
                })
                .ok()
            })
            .flatten()
    }

    /// Read the value. Panics if the value is no longer valid.
    pub fn read(&self) -> MappedRwLockReadGuard<'static, T> {
        self.try_read().unwrap()
    }

    /// Try to write the value. Returns None if the value is no longer valid.
    pub fn try_write(&self) -> Option<MappedRwLockWriteGuard<'static, T>> {
        self.validate()
            .then(|| {
                RwLockWriteGuard::try_map(self.raw.data.write(), |any| {
                    any.as_mut()?.downcast_mut::<T>()
                })
                .ok()
            })
            .flatten()
    }

    /// Write the value. Panics if the value is no longer valid.
    pub fn write(&self) -> MappedRwLockWriteGuard<'static, T> {
        self.try_write().unwrap()
    }

    /// Set the value. Panics if the value is no longer valid.
    pub fn set(&self, value: T) {
        self.validate().then(|| {
            *self.raw.data.write() = Some(Box::new(value));
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

impl<T> Copy for GenerationalBox<T> {}

impl<T> Clone for GenerationalBox<T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[derive(Clone, Copy)]
struct MemoryLocation {
    data: &'static RwLock<Option<Box<dyn std::any::Any>>>,
    #[cfg(any(debug_assertions, feature = "check_generation"))]
    generation: &'static AtomicU32,
}

impl MemoryLocation {
    #[allow(unused)]
    fn drop(&self) {
        let old = self.data.write().take();
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        if old.is_some() {
            drop(old);
            let new_generation = self.generation.load(std::sync::atomic::Ordering::Relaxed) + 1;
            self.generation
                .store(new_generation, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn replace<T: 'static>(&mut self, value: T) -> GenerationalBox<T> {
        let mut inner_mut = self.data.write();

        let raw = Box::new(value);
        let old = inner_mut.replace(raw);
        assert!(old.is_none());
        GenerationalBox {
            raw: *self,
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: self.generation.load(std::sync::atomic::Ordering::Relaxed),
            _marker: PhantomData,
        }
    }
}

/// Handles recycling generational boxes that have been dropped. Your application should have one store or one store per thread.
#[derive(Clone)]
pub struct Store {
    recycled: Arc<Mutex<Vec<MemoryLocation>>>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            recycled: Default::default(),
        }
    }
}

impl Store {
    fn recycle(&self, location: MemoryLocation) {
        location.drop();
        self.recycled.lock().push(location);
    }

    fn claim(&self) -> MemoryLocation {
        if let Some(location) = self.recycled.lock().pop() {
            location
        } else {
            let data: &'static RwLock<_> = Box::leak(Box::new(RwLock::new(None)));
            MemoryLocation {
                data,
                #[cfg(any(debug_assertions, feature = "check_generation"))]
                generation: Box::leak(Box::new(Default::default())),
            }
        }
    }

    /// Create a new owner. The owner will be responsible for dropping all of the generational boxes that it creates.
    pub fn owner(&self) -> Owner {
        Owner {
            store: self.clone(),
            owned: Default::default(),
        }
    }
}

/// Owner: Handles dropping generational boxes. The owner acts like a runtime lifetime guard. Any states that you create with an owner will be dropped when that owner is dropped.
pub struct Owner {
    store: Store,
    owned: Rc<RefCell<Vec<MemoryLocation>>>,
}

impl Owner {
    /// Insert a value into the store. The value will be dropped when the owner is dropped.
    pub fn insert<T: 'static>(&self, value: T) -> GenerationalBox<T> {
        let mut location = self.store.claim();
        let key = location.replace(value);
        self.owned.borrow_mut().push(location);
        key
    }

    /// Creates an invalid handle. This is useful for creating a handle that will be filled in later. If you use this before the value is filled in, you will get may get a panic or an out of date value.
    pub fn invalid<T: 'static>(&self) -> GenerationalBox<T> {
        let location = self.store.claim();
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

impl Drop for Owner {
    fn drop(&mut self) {
        for location in self.owned.borrow().iter() {
            self.store.recycle(*location)
        }
    }
}

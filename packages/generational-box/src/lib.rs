#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::{
    any::Any,
    cell::{Cell, Ref, RefCell, RefMut},
    error::Error,
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use bumpalo::Bump;

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
        first_ptr = owner.insert(1).raw.0.data.as_ptr();
        drop(owner);
    }
    {
        let owner = store.owner();
        let second_ptr = owner.insert(1234).raw.0.data.as_ptr();
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
    assert_eq!(
        key.try_read().as_deref().unwrap(),
        &"hello world".to_string()
    );
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
    assert!(key.try_read().is_err());
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
                println!("{:?}", &*value);
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
    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
    created_at: &'static std::panic::Location<'static>,
    _marker: PhantomData<T>,
}

impl<T: 'static> Debug for GenerationalBox<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        f.write_fmt(format_args!(
            "{:?}@{:?}",
            self.raw.0.data.as_ptr(),
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
            self.raw.0.generation.get() == self.generation
        }
        #[cfg(not(any(debug_assertions, feature = "check_generation")))]
        {
            true
        }
    }

    /// Try to read the value. Returns None if the value is no longer valid.
    #[track_caller]
    pub fn try_read(&self) -> Result<GenerationalRef<T>, BorrowError> {
        if !self.validate() {
            return Err(BorrowError::Dropped(ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                created_at: self.created_at,
            }));
        }
        self.raw.try_borrow(
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            self.created_at,
        )
    }

    /// Read the value. Panics if the value is no longer valid.
    #[track_caller]
    pub fn read(&self) -> GenerationalRef<T> {
        self.try_read().unwrap()
    }

    /// Try to write the value. Returns None if the value is no longer valid.
    #[track_caller]
    pub fn try_write(&self) -> Result<GenerationalRefMut<T>, BorrowMutError> {
        if !self.validate() {
            return Err(BorrowMutError::Dropped(ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                created_at: self.created_at,
            }));
        }
        self.raw.try_borrow_mut(
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            self.created_at,
        )
    }

    /// Write the value. Panics if the value is no longer valid.
    #[track_caller]
    pub fn write(&self) -> GenerationalRefMut<T> {
        self.try_write().unwrap()
    }

    /// Set the value. Panics if the value is no longer valid.
    pub fn set(&self, value: T) {
        self.validate().then(|| {
            *self.raw.0.data.borrow_mut() = Some(Box::new(value));
        });
    }

    /// Returns true if the pointer is equal to the other pointer.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        {
            self.raw.0.data.as_ptr() == other.raw.0.data.as_ptr()
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
struct MemoryLocation(&'static MemoryLocationInner);

struct MemoryLocationInner {
    data: RefCell<Option<Box<dyn std::any::Any>>>,
    #[cfg(any(debug_assertions, feature = "check_generation"))]
    generation: Cell<u32>,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrowed_at: RefCell<Vec<&'static std::panic::Location<'static>>>,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrowed_mut_at: Cell<Option<&'static std::panic::Location<'static>>>,
}

impl MemoryLocation {
    #[allow(unused)]
    fn drop(&self) {
        let old = self.0.data.borrow_mut().take();
        #[cfg(any(debug_assertions, feature = "check_generation"))]
        if old.is_some() {
            drop(old);
            let new_generation = self.0.generation.get() + 1;
            self.0.generation.set(new_generation);
        }
    }

    fn replace_with_caller<T: 'static>(
        &mut self,
        value: T,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalBox<T> {
        let mut inner_mut = self.0.data.borrow_mut();

        let raw = Box::new(value);
        let old = inner_mut.replace(raw);
        assert!(old.is_none());
        GenerationalBox {
            raw: *self,
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: self.0.generation.get(),
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
    ) -> Result<GenerationalRef<T>, BorrowError> {
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        self.0
            .borrowed_at
            .borrow_mut()
            .push(std::panic::Location::caller());
        match self.0.data.try_borrow() {
            Ok(borrow) => match Ref::filter_map(borrow, |any| any.as_ref()?.downcast_ref::<T>()) {
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
            },
            Err(_) => Err(BorrowError::AlreadyBorrowedMut(AlreadyBorrowedMutError {
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrowed_mut_at: self.0.borrowed_mut_at.get().unwrap(),
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
            self.0
                .borrowed_mut_at
                .set(Some(std::panic::Location::caller()));
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
                borrowed_at: self.0.borrowed_at.borrow().clone(),
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

/// Handles recycling generational boxes that have been dropped. Your application should have one store or one store per thread.
#[derive(Clone)]
pub struct Store {
    bump: &'static Bump,
    recycled: Rc<RefCell<Vec<MemoryLocation>>>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            bump: Box::leak(Box::new(Bump::new())),
            recycled: Default::default(),
        }
    }
}

impl Store {
    fn recycle(&self, location: MemoryLocation) {
        location.drop();
        self.recycled.borrow_mut().push(location);
    }

    fn claim(&self) -> MemoryLocation {
        if let Some(location) = self.recycled.borrow_mut().pop() {
            location
        } else {
            let data: &'static MemoryLocationInner = self.bump.alloc(MemoryLocationInner {
                data: RefCell::new(None),
                #[cfg(any(debug_assertions, feature = "check_generation"))]
                generation: Cell::new(0),
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrowed_at: Default::default(),
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrowed_mut_at: Default::default(),
            });
            MemoryLocation(data)
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
    #[track_caller]
    pub fn insert<T: 'static>(&self, value: T) -> GenerationalBox<T> {
        let mut location = self.store.claim();
        let key = location.replace_with_caller(
            value,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            std::panic::Location::caller(),
        );
        self.owned.borrow_mut().push(location);
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
    pub fn invalid<T: 'static>(&self) -> GenerationalBox<T> {
        let location = self.store.claim();
        let key = GenerationalBox {
            raw: location,
            #[cfg(any(debug_assertions, feature = "check_generation"))]
            generation: location.0.generation.get(),
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            created_at: std::panic::Location::caller(),
            _marker: PhantomData,
        };
        self.owned.borrow_mut().push(location);
        key
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        for location in self.owned.borrow().iter() {
            self.store.recycle(*location)
        }
    }
}

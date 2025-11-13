use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::{
    any::Any,
    fmt::Debug,
    num::NonZeroU64,
    sync::{Arc, OnceLock},
};

use crate::{
    entry::{MemoryLocationBorrowInfo, RcStorageEntry, StorageEntry},
    error::{self, ValueDroppedError},
    references::{GenerationalRef, GenerationalRefMut},
    AnyStorage, BorrowError, BorrowMutError, BorrowMutResult, BorrowResult, GenerationalLocation,
    GenerationalPointer, Storage,
};

type RwLockStorageEntryRef = RwLockReadGuard<'static, StorageEntry<RwLockStorageEntryData>>;
type RwLockStorageEntryMut = RwLockWriteGuard<'static, StorageEntry<RwLockStorageEntryData>>;

type AnyRef = MappedRwLockReadGuard<'static, Box<dyn Any + Send + Sync + 'static>>;
type AnyRefMut = MappedRwLockWriteGuard<'static, Box<dyn Any + Send + Sync + 'static>>;

#[derive(Default)]
pub(crate) enum RwLockStorageEntryData {
    Reference(GenerationalPointer<SyncStorage>),
    Rc(RcStorageEntry<Box<dyn Any + Send + Sync>>),
    Data(Box<dyn Any + Send + Sync>),
    #[default]
    Empty,
}

impl Debug for RwLockStorageEntryData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reference(location) => write!(f, "Reference({location:?})"),
            Self::Rc(_) => write!(f, "Rc"),
            Self::Data(_) => write!(f, "Data"),
            Self::Empty => write!(f, "Empty"),
        }
    }
}

impl RwLockStorageEntryData {
    pub const fn new_full(data: Box<dyn Any + Send + Sync>) -> Self {
        Self::Data(data)
    }
}

/// A thread safe storage. This is slower than the unsync storage, but allows you to share the value between threads.
#[derive(Default)]
pub struct SyncStorage {
    borrow_info: MemoryLocationBorrowInfo,
    data: RwLock<StorageEntry<RwLockStorageEntryData>>,
}

impl SyncStorage {
    pub(crate) fn read(
        pointer: GenerationalPointer<Self>,
    ) -> BorrowResult<(AnyRef, GenerationalPointer<Self>)> {
        Self::get_split_ref(pointer).map(|(resolved, guard)| {
            (
                RwLockReadGuard::map(guard, |data| match &data.data {
                    RwLockStorageEntryData::Data(data) => data,
                    RwLockStorageEntryData::Rc(data) => &data.data,
                    _ => unreachable!(),
                }),
                resolved,
            )
        })
    }

    pub(crate) fn get_split_ref(
        mut pointer: GenerationalPointer<Self>,
    ) -> BorrowResult<(GenerationalPointer<Self>, RwLockStorageEntryRef)> {
        loop {
            let borrow = pointer.storage.data.read();
            if !borrow.valid(&pointer.location) {
                return Err(BorrowError::Dropped(ValueDroppedError::new_for_location(
                    pointer.location,
                )));
            }
            match &borrow.data {
                // If this is a reference, keep traversing the pointers
                RwLockStorageEntryData::Reference(data) => {
                    pointer = *data;
                }
                // Otherwise return the value
                RwLockStorageEntryData::Data(_) | RwLockStorageEntryData::Rc(_) => {
                    return Ok((pointer, borrow));
                }
                RwLockStorageEntryData::Empty => {
                    return Err(BorrowError::Dropped(ValueDroppedError::new_for_location(
                        pointer.location,
                    )));
                }
            }
        }
    }

    pub(crate) fn write(
        pointer: GenerationalPointer<Self>,
    ) -> BorrowMutResult<(AnyRefMut, GenerationalPointer<Self>)> {
        Self::get_split_mut(pointer).map(|(resolved, guard)| {
            (
                RwLockWriteGuard::map(guard, |data| match &mut data.data {
                    RwLockStorageEntryData::Data(data) => data,
                    RwLockStorageEntryData::Rc(data) => &mut data.data,
                    _ => unreachable!(),
                }),
                resolved,
            )
        })
    }

    pub(crate) fn get_split_mut(
        mut pointer: GenerationalPointer<Self>,
    ) -> BorrowMutResult<(GenerationalPointer<Self>, RwLockStorageEntryMut)> {
        loop {
            let borrow = pointer.storage.data.write();
            if !borrow.valid(&pointer.location) {
                return Err(BorrowMutError::Dropped(
                    ValueDroppedError::new_for_location(pointer.location),
                ));
            }
            match &borrow.data {
                // If this is a reference, keep traversing the pointers
                RwLockStorageEntryData::Reference(data) => {
                    pointer = *data;
                }
                // Otherwise return the value
                RwLockStorageEntryData::Data(_) | RwLockStorageEntryData::Rc(_) => {
                    return Ok((pointer, borrow));
                }
                RwLockStorageEntryData::Empty => {
                    return Err(BorrowMutError::Dropped(
                        ValueDroppedError::new_for_location(pointer.location),
                    ));
                }
            }
        }
    }

    fn create_new(
        value: RwLockStorageEntryData,
        #[allow(unused)] caller: &'static std::panic::Location<'static>,
    ) -> GenerationalPointer<Self> {
        match sync_runtime().lock().pop() {
            Some(storage) => {
                let mut write = storage.data.write();
                let location = GenerationalLocation {
                    generation: write.generation(),
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    created_at: caller,
                };
                write.data = value;
                GenerationalPointer { storage, location }
            }
            None => {
                let storage: &'static Self = &*Box::leak(Box::new(Self {
                    borrow_info: Default::default(),
                    data: RwLock::new(StorageEntry::new(value)),
                }));

                let location = GenerationalLocation {
                    generation: NonZeroU64::MIN,
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    created_at: caller,
                };

                GenerationalPointer { storage, location }
            }
        }
    }
}

static SYNC_RUNTIME: OnceLock<Arc<Mutex<Vec<&'static SyncStorage>>>> = OnceLock::new();

fn sync_runtime() -> &'static Arc<Mutex<Vec<&'static SyncStorage>>> {
    SYNC_RUNTIME.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

impl AnyStorage for SyncStorage {
    type Ref<'a, R: ?Sized + 'a> = GenerationalRef<MappedRwLockReadGuard<'a, R>>;
    type Mut<'a, W: ?Sized + 'a> = GenerationalRefMut<MappedRwLockWriteGuard<'a, W>>;

    fn downcast_lifetime_ref<'a: 'b, 'b, T: ?Sized + 'b>(
        ref_: Self::Ref<'a, T>,
    ) -> Self::Ref<'b, T> {
        ref_
    }

    fn downcast_lifetime_mut<'a: 'b, 'b, T: ?Sized + 'a>(
        mut_: Self::Mut<'a, T>,
    ) -> Self::Mut<'b, T> {
        mut_
    }

    fn map<T: ?Sized, U: ?Sized>(
        ref_: Self::Ref<'_, T>,
        f: impl FnOnce(&T) -> &U,
    ) -> Self::Ref<'_, U> {
        ref_.map(|inner| MappedRwLockReadGuard::map(inner, f))
    }

    fn map_mut<T: ?Sized, U: ?Sized>(
        mut_ref: Self::Mut<'_, T>,
        f: impl FnOnce(&mut T) -> &mut U,
    ) -> Self::Mut<'_, U> {
        mut_ref.map(|inner| MappedRwLockWriteGuard::map(inner, f))
    }

    fn try_map<I: ?Sized, U: ?Sized>(
        ref_: Self::Ref<'_, I>,
        f: impl FnOnce(&I) -> Option<&U>,
    ) -> Option<Self::Ref<'_, U>> {
        ref_.try_map(|inner| MappedRwLockReadGuard::try_map(inner, f).ok())
    }

    fn try_map_mut<I: ?Sized, U: ?Sized>(
        mut_ref: Self::Mut<'_, I>,
        f: impl FnOnce(&mut I) -> Option<&mut U>,
    ) -> Option<Self::Mut<'_, U>> {
        mut_ref.try_map(|inner| MappedRwLockWriteGuard::try_map(inner, f).ok())
    }

    fn data_ptr(&self) -> *const () {
        self.data.data_ptr() as *const ()
    }

    fn recycle(pointer: GenerationalPointer<Self>) {
        let mut borrow_mut = pointer.storage.data.write();

        // First check if the generation is still valid
        if !borrow_mut.valid(&pointer.location) {
            return;
        }

        borrow_mut.increment_generation();

        // Then decrement the reference count or drop the value if it's the last reference
        match &mut borrow_mut.data {
            // If this is the original reference, drop the value
            RwLockStorageEntryData::Data(_) => borrow_mut.data = RwLockStorageEntryData::Empty,
            // If this is a rc, just ignore the drop
            RwLockStorageEntryData::Rc(_) => {}
            // If this is a reference, decrement the reference count
            RwLockStorageEntryData::Reference(reference) => {
                drop_ref(*reference);
            }
            RwLockStorageEntryData::Empty => {}
        }

        sync_runtime().lock().push(pointer.storage);
    }
}

fn drop_ref(pointer: GenerationalPointer<SyncStorage>) {
    let mut borrow_mut = pointer.storage.data.write();

    // First check if the generation is still valid
    if !borrow_mut.valid(&pointer.location) {
        return;
    }

    if let RwLockStorageEntryData::Rc(entry) = &mut borrow_mut.data {
        // Decrement the reference count
        if entry.drop_ref() {
            // If the reference count is now zero, drop the value
            borrow_mut.data = RwLockStorageEntryData::Empty;
            sync_runtime().lock().push(pointer.storage);
        }
    } else {
        unreachable!("References should always point to a data entry directly");
    }
}

impl<T: Sync + Send + 'static> Storage<T> for SyncStorage {
    #[track_caller]
    fn try_read(
        pointer: GenerationalPointer<Self>,
    ) -> Result<Self::Ref<'static, T>, error::BorrowError> {
        let (read, pointer) = Self::read(pointer)?;

        let read = MappedRwLockReadGuard::try_map(read, |any| {
            // Then try to downcast
            any.downcast_ref()
        });
        match read {
            Ok(guard) => Ok(GenerationalRef::new(
                guard,
                pointer.storage.borrow_info.borrow_guard(),
            )),
            Err(_) => Err(error::BorrowError::Dropped(
                ValueDroppedError::new_for_location(pointer.location),
            )),
        }
    }

    #[track_caller]
    fn try_write(
        pointer: GenerationalPointer<Self>,
    ) -> Result<Self::Mut<'static, T>, error::BorrowMutError> {
        let (write, pointer) = Self::write(pointer)?;

        let write = MappedRwLockWriteGuard::try_map(write, |any| {
            // Then try to downcast
            any.downcast_mut()
        });
        match write {
            Ok(guard) => Ok(GenerationalRefMut::new(
                guard,
                pointer.storage.borrow_info.borrow_mut_guard(),
            )),
            Err(_) => Err(error::BorrowMutError::Dropped(
                ValueDroppedError::new_for_location(pointer.location),
            )),
        }
    }

    fn new(value: T, caller: &'static std::panic::Location<'static>) -> GenerationalPointer<Self> {
        Self::create_new(RwLockStorageEntryData::new_full(Box::new(value)), caller)
    }

    fn new_rc(
        value: T,
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalPointer<Self> {
        // Create the data that the rc points to
        let data = Self::create_new(
            RwLockStorageEntryData::Rc(RcStorageEntry::new(Box::new(value))),
            caller,
        );
        Self::create_new(RwLockStorageEntryData::Reference(data), caller)
    }

    fn new_reference(
        location: GenerationalPointer<Self>,
    ) -> BorrowResult<GenerationalPointer<Self>> {
        // Chase the reference to get the final location
        let (location, value) = Self::get_split_ref(location)?;
        if let RwLockStorageEntryData::Rc(data) = &value.data {
            data.add_ref();
        } else {
            unreachable!()
        }
        Ok(Self::create_new(
            RwLockStorageEntryData::Reference(location),
            location
                .location
                .created_at()
                .unwrap_or(std::panic::Location::caller()),
        ))
    }

    fn change_reference(
        location: GenerationalPointer<Self>,
        other: GenerationalPointer<Self>,
    ) -> BorrowResult {
        if location == other {
            return Ok(());
        }

        let (other_final, other_write) = Self::get_split_ref(other)?;

        let mut write = location.storage.data.write();
        // First check if the generation is still valid
        if !write.valid(&location.location) {
            return Err(BorrowError::Dropped(ValueDroppedError::new_for_location(
                location.location,
            )));
        }

        if let (RwLockStorageEntryData::Reference(reference), RwLockStorageEntryData::Rc(data)) =
            (&mut write.data, &other_write.data)
        {
            if reference == &other_final {
                return Ok(());
            }
            drop_ref(*reference);
            *reference = other_final;
            data.add_ref();
        } else {
            tracing::trace!(
                "References should always point to a data entry directly found {:?} instead",
                other_write.data
            );
            return Err(BorrowError::Dropped(ValueDroppedError::new_for_location(
                other_final.location,
            )));
        }

        Ok(())
    }
}

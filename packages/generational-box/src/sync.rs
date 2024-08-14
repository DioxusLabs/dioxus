use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::{
    any::Any,
    num::NonZeroU64,
    sync::{Arc, OnceLock},
};

use crate::{
    entry::{FullStorageEntry, MemoryLocationBorrowInfo, StorageEntry},
    error::{self, ValueDroppedError},
    references::{GenerationalRef, GenerationalRefMut},
    AnyStorage, BorrowError, BorrowMutError, GenerationalLocation, GenerationalPointer, Storage,
};

pub(crate) enum RwLockStorageEntryData<T: 'static> {
    Reference(GenerationalPointer<SyncStorage>),
    Data(FullStorageEntry<T>),
    Empty,
}

impl<T: 'static> Default for RwLockStorageEntryData<T> {
    fn default() -> Self {
        Self::Empty
    }
}

impl<T> RwLockStorageEntryData<T> {
    pub const fn new_full(data: T) -> Self {
        Self::Data(FullStorageEntry::new(data))
    }
}

/// A thread safe storage. This is slower than the unsync storage, but allows you to share the value between threads.
#[derive(Default)]
pub struct SyncStorage {
    borrow_info: MemoryLocationBorrowInfo,
    data: RwLock<StorageEntry<RwLockStorageEntryData<Box<dyn Any + Send + Sync>>>>,
}

impl SyncStorage {
    pub(crate) fn read(
        pointer: GenerationalPointer<Self>,
    ) -> Result<
        MappedRwLockReadGuard<'static, FullStorageEntry<Box<dyn Any + Send + Sync>>>,
        BorrowError,
    > {
        Self::get_split_ref(pointer).map(|(_, guard)| guard)
    }

    pub(crate) fn get_split_ref(
        mut pointer: GenerationalPointer<Self>,
    ) -> Result<
        (
            GenerationalPointer<Self>,
            MappedRwLockReadGuard<'static, FullStorageEntry<Box<dyn Any + Send + Sync>>>,
        ),
        BorrowError,
    > {
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
                RwLockStorageEntryData::Data(_) => {
                    return Ok((
                        pointer,
                        RwLockReadGuard::map(borrow, |data| {
                            if let RwLockStorageEntryData::Data(data) = &data.data {
                                data
                            } else {
                                unreachable!()
                            }
                        }),
                    ));
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
    ) -> Result<
        MappedRwLockWriteGuard<'static, FullStorageEntry<Box<dyn Any + Send + Sync>>>,
        BorrowMutError,
    > {
        Self::get_split_mut(pointer).map(|(_, guard)| guard)
    }

    pub(crate) fn get_split_mut(
        mut pointer: GenerationalPointer<Self>,
    ) -> Result<
        (
            GenerationalPointer<Self>,
            MappedRwLockWriteGuard<'static, FullStorageEntry<Box<dyn Any + Send + Sync>>>,
        ),
        BorrowMutError,
    > {
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
                RwLockStorageEntryData::Data(_) => {
                    return Ok((
                        pointer,
                        RwLockWriteGuard::map(borrow, |data| {
                            if let RwLockStorageEntryData::Data(data) = &mut data.data {
                                data
                            } else {
                                unreachable!()
                            }
                        }),
                    ));
                }
                RwLockStorageEntryData::Empty => {
                    return Err(BorrowMutError::Dropped(
                        ValueDroppedError::new_for_location(pointer.location),
                    ));
                }
            }
        }
    }
}

static SYNC_RUNTIME: OnceLock<Arc<Mutex<Vec<&'static SyncStorage>>>> = OnceLock::new();

fn sync_runtime() -> &'static Arc<Mutex<Vec<&'static SyncStorage>>> {
    SYNC_RUNTIME.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

impl AnyStorage for SyncStorage {
    type Ref<'a, R: ?Sized + 'static> = GenerationalRef<MappedRwLockReadGuard<'a, R>>;
    type Mut<'a, W: ?Sized + 'static> = GenerationalRefMut<MappedRwLockWriteGuard<'a, W>>;

    fn downcast_lifetime_ref<'a: 'b, 'b, T: ?Sized + 'static>(
        ref_: Self::Ref<'a, T>,
    ) -> Self::Ref<'b, T> {
        ref_
    }

    fn downcast_lifetime_mut<'a: 'b, 'b, T: ?Sized + 'static>(
        mut_: Self::Mut<'a, T>,
    ) -> Self::Mut<'b, T> {
        mut_
    }

    fn map<T: ?Sized + 'static, U: ?Sized + 'static>(
        ref_: Self::Ref<'_, T>,
        f: impl FnOnce(&T) -> &U,
    ) -> Self::Ref<'_, U> {
        ref_.map(|inner| MappedRwLockReadGuard::map(inner, f))
    }

    fn map_mut<T: ?Sized + 'static, U: ?Sized + 'static>(
        mut_ref: Self::Mut<'_, T>,
        f: impl FnOnce(&mut T) -> &mut U,
    ) -> Self::Mut<'_, U> {
        mut_ref.map(|inner| MappedRwLockWriteGuard::map(inner, f))
    }

    fn try_map<I: ?Sized + 'static, U: ?Sized + 'static>(
        ref_: Self::Ref<'_, I>,
        f: impl FnOnce(&I) -> Option<&U>,
    ) -> Option<Self::Ref<'_, U>> {
        ref_.try_map(|inner| MappedRwLockReadGuard::try_map(inner, f).ok())
    }

    fn try_map_mut<I: ?Sized + 'static, U: ?Sized + 'static>(
        mut_ref: Self::Mut<'_, I>,
        f: impl FnOnce(&mut I) -> Option<&mut U>,
    ) -> Option<Self::Mut<'_, U>> {
        mut_ref.try_map(|inner| MappedRwLockWriteGuard::try_map(inner, f).ok())
    }

    fn data_ptr(&self) -> *const () {
        self.data.data_ptr() as *const ()
    }

    #[track_caller]
    #[allow(unused)]
    fn claim(caller: &'static std::panic::Location<'static>) -> GenerationalPointer<Self> {
        match sync_runtime().lock().pop() {
            Some(mut storage) => {
                let location = GenerationalLocation {
                    generation: storage.data.read().generation(),
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    created_at: caller,
                };
                GenerationalPointer { storage, location }
            }
            None => {
                let storage: &'static Self = &*Box::leak(Box::default());

                let location = GenerationalLocation {
                    generation: NonZeroU64::MIN,
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    created_at: caller,
                };

                GenerationalPointer { storage, location }
            }
        }
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

    if let RwLockStorageEntryData::Data(entry) = &mut borrow_mut.data {
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
        let read = Self::read(pointer)?;

        let read = MappedRwLockReadGuard::try_map(read, |any| {
            // Then try to downcast
            any.data.downcast_ref()
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
        let write = Self::write(pointer)?;

        let write = MappedRwLockWriteGuard::try_map(write, |any| {
            // Then try to downcast
            any.data.downcast_mut()
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

    fn set(pointer: GenerationalPointer<Self>, value: T) {
        let mut write = pointer.storage.data.write();
        // First check if the generation is still valid
        if !write.valid(&pointer.location) {
            return;
        }
        write.data =
            RwLockStorageEntryData::new_full(Box::new(value) as Box<dyn Any + Send + Sync>);
    }

    fn reference(
        location: GenerationalPointer<Self>,
        other: GenerationalPointer<Self>,
    ) -> Result<(), BorrowMutError> {
        let (other_final, mut other_write) = Self::get_split_mut(other)?;

        let mut write = location.storage.data.write();
        // First check if the generation is still valid
        if !write.valid(&location.location) {
            return Err(BorrowMutError::Dropped(
                ValueDroppedError::new_for_location(location.location),
            ));
        }

        match &mut write.data {
            RwLockStorageEntryData::Reference(reference) => {
                drop_ref(*reference);
                *reference = other_final;
            }
            RwLockStorageEntryData::Data(_) => {}
            RwLockStorageEntryData::Empty => {
                // Just point to the other location
                write.data = RwLockStorageEntryData::Reference(other_final);
            }
        }
        other_write.add_ref();

        Ok(())
    }
}

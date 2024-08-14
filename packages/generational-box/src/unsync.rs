use crate::{
    entry::{FullStorageEntry, MemoryLocationBorrowInfo, StorageEntry},
    error,
    references::{GenerationalRef, GenerationalRefMut},
    AnyStorage, BorrowError, BorrowMutError, GenerationalLocation, GenerationalPointer, Storage,
    ValueDroppedError,
};
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    num::NonZeroU64,
};

thread_local! {
    static UNSYNC_RUNTIME: RefCell<Vec<&'static UnsyncStorage>> = const { RefCell::new(Vec::new()) };
}

pub(crate) enum RefCellStorageEntryData<T: 'static> {
    Reference(GenerationalPointer<UnsyncStorage>),
    Data(FullStorageEntry<T>),
    Empty,
}

impl<T: 'static> Default for RefCellStorageEntryData<T> {
    fn default() -> Self {
        Self::Empty
    }
}

impl<T> RefCellStorageEntryData<T> {
    pub const fn new_full(data: T) -> Self {
        Self::Data(FullStorageEntry::new(data))
    }
}

/// A unsync storage. This is the default storage type.
#[derive(Default)]
pub struct UnsyncStorage {
    borrow_info: MemoryLocationBorrowInfo,
    data: RefCell<StorageEntry<RefCellStorageEntryData<Box<dyn Any>>>>,
}

impl UnsyncStorage {
    pub(crate) fn read(
        pointer: GenerationalPointer<Self>,
    ) -> Result<Ref<'static, FullStorageEntry<Box<dyn Any>>>, BorrowError> {
        Self::get_split_ref(pointer).map(|(_, guard)| guard)
    }

    pub(crate) fn get_split_ref(
        mut pointer: GenerationalPointer<Self>,
    ) -> Result<
        (
            GenerationalPointer<Self>,
            Ref<'static, FullStorageEntry<Box<dyn Any>>>,
        ),
        BorrowError,
    > {
        loop {
            let borrow = pointer
                .storage
                .data
                .try_borrow()
                .map_err(|_| pointer.storage.borrow_info.borrow_error())?;
            if !borrow.valid(&pointer.location) {
                return Err(BorrowError::Dropped(ValueDroppedError::new_for_location(
                    pointer.location,
                )));
            }
            match &borrow.data {
                // If this is a reference, keep traversing the pointers
                RefCellStorageEntryData::Reference(data) => {
                    pointer = *data;
                }
                // Otherwise return the value
                RefCellStorageEntryData::Data(_) => {
                    return Ok((
                        pointer,
                        Ref::map(borrow, |data| {
                            if let RefCellStorageEntryData::Data(data) = &data.data {
                                data
                            } else {
                                unreachable!()
                            }
                        }),
                    ));
                }
                RefCellStorageEntryData::Empty => {
                    return Err(BorrowError::Dropped(ValueDroppedError::new_for_location(
                        pointer.location,
                    )));
                }
            }
        }
    }

    pub(crate) fn write(
        pointer: GenerationalPointer<Self>,
    ) -> Result<RefMut<'static, FullStorageEntry<Box<dyn Any>>>, BorrowMutError> {
        Self::get_split_mut(pointer).map(|(_, guard)| guard)
    }

    pub(crate) fn get_split_mut(
        mut pointer: GenerationalPointer<Self>,
    ) -> Result<
        (
            GenerationalPointer<Self>,
            RefMut<'static, FullStorageEntry<Box<dyn Any>>>,
        ),
        BorrowMutError,
    > {
        loop {
            let borrow = pointer
                .storage
                .data
                .try_borrow_mut()
                .map_err(|_| pointer.storage.borrow_info.borrow_mut_error())?;
            if !borrow.valid(&pointer.location) {
                return Err(BorrowMutError::Dropped(
                    ValueDroppedError::new_for_location(pointer.location),
                ));
            }
            match &borrow.data {
                // If this is a reference, keep traversing the pointers
                RefCellStorageEntryData::Reference(data) => {
                    pointer = *data;
                }
                // Otherwise return the value
                RefCellStorageEntryData::Data(_) => {
                    return Ok((
                        pointer,
                        RefMut::map(borrow, |data| {
                            if let RefCellStorageEntryData::Data(data) = &mut data.data {
                                data
                            } else {
                                unreachable!()
                            }
                        }),
                    ));
                }
                RefCellStorageEntryData::Empty => {
                    return Err(BorrowMutError::Dropped(
                        ValueDroppedError::new_for_location(pointer.location),
                    ));
                }
            }
        }
    }
}

impl AnyStorage for UnsyncStorage {
    type Ref<'a, R: ?Sized + 'static> = GenerationalRef<Ref<'a, R>>;
    type Mut<'a, W: ?Sized + 'static> = GenerationalRefMut<RefMut<'a, W>>;

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
        ref_.map(|inner| Ref::map(inner, f))
    }

    fn map_mut<T: ?Sized + 'static, U: ?Sized + 'static>(
        mut_ref: Self::Mut<'_, T>,
        f: impl FnOnce(&mut T) -> &mut U,
    ) -> Self::Mut<'_, U> {
        mut_ref.map(|inner| RefMut::map(inner, f))
    }

    fn try_map<I: ?Sized + 'static, U: ?Sized + 'static>(
        _self: Self::Ref<'_, I>,
        f: impl FnOnce(&I) -> Option<&U>,
    ) -> Option<Self::Ref<'_, U>> {
        _self.try_map(|inner| Ref::filter_map(inner, f).ok())
    }

    fn try_map_mut<I: ?Sized + 'static, U: ?Sized + 'static>(
        mut_ref: Self::Mut<'_, I>,
        f: impl FnOnce(&mut I) -> Option<&mut U>,
    ) -> Option<Self::Mut<'_, U>> {
        mut_ref.try_map(|inner| RefMut::filter_map(inner, f).ok())
    }

    fn data_ptr(&self) -> *const () {
        self.data.as_ptr() as *const ()
    }

    #[allow(unused)]
    fn claim(caller: &'static std::panic::Location<'static>) -> GenerationalPointer<Self> {
        UNSYNC_RUNTIME.with(|runtime| {
            if let Some(storage) = runtime.borrow_mut().pop() {
                let generation = storage.data.borrow().generation();
                let location = GenerationalLocation {
                    generation,
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    created_at: caller,
                };
                GenerationalPointer { storage, location }
            } else {
                let data: &'static Self = &*Box::leak(Box::default());
                let location = GenerationalLocation {
                    generation: NonZeroU64::MIN,
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    created_at: caller,
                };
                GenerationalPointer {
                    storage: data,
                    location,
                }
            }
        })
    }

    fn recycle(pointer: GenerationalPointer<Self>) {
        let mut borrow_mut = pointer.storage.data.borrow_mut();

        // First check if the generation is still valid
        if !borrow_mut.valid(&pointer.location) {
            return;
        }

        borrow_mut.increment_generation();
        // Then decrement the reference count or drop the value if it's the last reference
        match &mut borrow_mut.data {
            // If this is the original reference, drop the value
            RefCellStorageEntryData::Data(_) => borrow_mut.data = RefCellStorageEntryData::Empty,
            // If this is a reference, decrement the reference count
            RefCellStorageEntryData::Reference(reference) => {
                drop_ref(*reference);
            }
            RefCellStorageEntryData::Empty => {}
        }

        UNSYNC_RUNTIME.with(|runtime| runtime.borrow_mut().push(pointer.storage));
    }
}

fn drop_ref(pointer: GenerationalPointer<UnsyncStorage>) {
    let mut borrow_mut = pointer.storage.data.borrow_mut();

    // First check if the generation is still valid
    if !borrow_mut.valid(&pointer.location) {
        return;
    }

    if let RefCellStorageEntryData::Data(entry) = &mut borrow_mut.data {
        // Decrement the reference count
        if entry.drop_ref() {
            // If the reference count is now zero, drop the value
            borrow_mut.data = RefCellStorageEntryData::Empty;
            UNSYNC_RUNTIME.with(|runtime| runtime.borrow_mut().push(pointer.storage));
        }
    } else {
        unreachable!("References should always point to a data entry directly");
    }
}

impl<T: 'static> Storage<T> for UnsyncStorage {
    #[track_caller]
    fn try_read(
        pointer: GenerationalPointer<Self>,
    ) -> Result<Self::Ref<'static, T>, error::BorrowError> {
        let read = Self::read(pointer)?;

        let ref_ = Ref::filter_map(read, |any| {
            // Then try to downcast
            any.data.downcast_ref()
        });
        match ref_ {
            Ok(guard) => Ok(GenerationalRef::new(
                guard,
                pointer.storage.borrow_info.borrow_guard(),
            )),
            Err(_) => Err(error::BorrowError::Dropped(
                error::ValueDroppedError::new_for_location(pointer.location),
            )),
        }
    }

    #[track_caller]
    fn try_write(
        pointer: GenerationalPointer<Self>,
    ) -> Result<Self::Mut<'static, T>, error::BorrowMutError> {
        let write = Self::write(pointer)?;

        let ref_mut = RefMut::filter_map(write, |any| {
            // Then try to downcast
            any.data.downcast_mut()
        });
        match ref_mut {
            Ok(guard) => Ok(GenerationalRefMut::new(
                guard,
                pointer.storage.borrow_info.borrow_mut_guard(),
            )),
            Err(_) => Err(error::BorrowMutError::Dropped(
                error::ValueDroppedError::new_for_location(pointer.location),
            )),
        }
    }

    fn set(pointer: GenerationalPointer<Self>, value: T) {
        let mut borrow_mut = pointer.storage.data.borrow_mut();
        // First check if the generation is still valid
        if !borrow_mut.valid(&pointer.location) {
            return;
        }
        borrow_mut.data = RefCellStorageEntryData::new_full(Box::new(value) as Box<dyn Any>);
    }

    fn reference(
        location: GenerationalPointer<Self>,
        other: GenerationalPointer<Self>,
    ) -> Result<(), BorrowMutError> {
        let (other_final, mut other_write) = Self::get_split_mut(other)?;

        let mut write = location.storage.data.borrow_mut();
        // First check if the generation is still valid
        if !write.valid(&location.location) {
            return Err(BorrowMutError::Dropped(
                ValueDroppedError::new_for_location(location.location),
            ));
        }

        match &mut write.data {
            RefCellStorageEntryData::Reference(reference) => {
                drop_ref(*reference);
                *reference = other_final;
            }
            RefCellStorageEntryData::Data(_) | RefCellStorageEntryData::Empty => {
                // Just point to the other location
                write.data = RefCellStorageEntryData::Reference(other_final);
            }
        }
        other_write.add_ref();

        Ok(())
    }
}

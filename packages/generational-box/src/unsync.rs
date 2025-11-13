use crate::{
    entry::{MemoryLocationBorrowInfo, RcStorageEntry, StorageEntry},
    error,
    references::{GenerationalRef, GenerationalRefMut},
    AnyStorage, BorrowError, BorrowMutError, BorrowMutResult, BorrowResult, GenerationalLocation,
    GenerationalPointer, Storage, ValueDroppedError,
};
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    num::NonZeroU64,
};

type RefCellStorageEntryRef = Ref<'static, StorageEntry<RefCellStorageEntryData>>;
type RefCellStorageEntryMut = RefMut<'static, StorageEntry<RefCellStorageEntryData>>;
type AnyRef = Ref<'static, Box<dyn Any>>;
type AnyRefMut = RefMut<'static, Box<dyn Any>>;

thread_local! {
    static UNSYNC_RUNTIME: RefCell<Vec<&'static UnsyncStorage>> = const { RefCell::new(Vec::new()) };
}

#[derive(Default)]
pub(crate) enum RefCellStorageEntryData {
    Reference(GenerationalPointer<UnsyncStorage>),
    Rc(RcStorageEntry<Box<dyn Any>>),
    Data(Box<dyn Any>),
    #[default]
    Empty,
}

impl Debug for RefCellStorageEntryData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reference(pointer) => write!(f, "Reference({:?})", pointer.location),
            Self::Rc(_) => write!(f, "Rc"),
            Self::Data(_) => write!(f, "Data"),
            Self::Empty => write!(f, "Empty"),
        }
    }
}

/// A unsync storage. This is the default storage type.
#[derive(Default)]
pub struct UnsyncStorage {
    borrow_info: MemoryLocationBorrowInfo,
    data: RefCell<StorageEntry<RefCellStorageEntryData>>,
}

impl UnsyncStorage {
    pub(crate) fn read(
        pointer: GenerationalPointer<Self>,
    ) -> BorrowResult<(AnyRef, GenerationalPointer<Self>)> {
        Self::get_split_ref(pointer).map(|(resolved, guard)| {
            (
                Ref::map(guard, |data| match &data.data {
                    RefCellStorageEntryData::Data(data) => data,
                    RefCellStorageEntryData::Rc(data) => &data.data,
                    _ => unreachable!(),
                }),
                resolved,
            )
        })
    }

    pub(crate) fn get_split_ref(
        mut pointer: GenerationalPointer<Self>,
    ) -> BorrowResult<(GenerationalPointer<Self>, RefCellStorageEntryRef)> {
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
                RefCellStorageEntryData::Rc(_) | RefCellStorageEntryData::Data(_) => {
                    return Ok((pointer, borrow));
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
    ) -> BorrowMutResult<(AnyRefMut, GenerationalPointer<Self>)> {
        Self::get_split_mut(pointer).map(|(resolved, guard)| {
            (
                RefMut::map(guard, |data| match &mut data.data {
                    RefCellStorageEntryData::Data(data) => data,
                    RefCellStorageEntryData::Rc(data) => &mut data.data,
                    _ => unreachable!(),
                }),
                resolved,
            )
        })
    }

    pub(crate) fn get_split_mut(
        mut pointer: GenerationalPointer<Self>,
    ) -> BorrowMutResult<(GenerationalPointer<Self>, RefCellStorageEntryMut)> {
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
                RefCellStorageEntryData::Data(_) | RefCellStorageEntryData::Rc(_) => {
                    return Ok((pointer, borrow));
                }
                RefCellStorageEntryData::Empty => {
                    return Err(BorrowMutError::Dropped(
                        ValueDroppedError::new_for_location(pointer.location),
                    ));
                }
            }
        }
    }

    fn create_new(
        value: RefCellStorageEntryData,
        #[allow(unused)] caller: &'static std::panic::Location<'static>,
    ) -> GenerationalPointer<Self> {
        UNSYNC_RUNTIME.with(|runtime| match runtime.borrow_mut().pop() {
            Some(storage) => {
                let mut write = storage.data.borrow_mut();
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
                    data: RefCell::new(StorageEntry::new(value)),
                }));

                let location = GenerationalLocation {
                    generation: NonZeroU64::MIN,
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    created_at: caller,
                };

                GenerationalPointer { storage, location }
            }
        })
    }
}

impl AnyStorage for UnsyncStorage {
    type Ref<'a, R: ?Sized + 'a> = GenerationalRef<Ref<'a, R>>;
    type Mut<'a, W: ?Sized + 'a> = GenerationalRefMut<RefMut<'a, W>>;

    fn downcast_lifetime_ref<'a: 'b, 'b, T: ?Sized + 'a>(
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
        ref_.map(|inner| Ref::map(inner, f))
    }

    fn map_mut<T: ?Sized, U: ?Sized>(
        mut_ref: Self::Mut<'_, T>,
        f: impl FnOnce(&mut T) -> &mut U,
    ) -> Self::Mut<'_, U> {
        mut_ref.map(|inner| RefMut::map(inner, f))
    }

    fn try_map<I: ?Sized, U: ?Sized>(
        _self: Self::Ref<'_, I>,
        f: impl FnOnce(&I) -> Option<&U>,
    ) -> Option<Self::Ref<'_, U>> {
        _self.try_map(|inner| Ref::filter_map(inner, f).ok())
    }

    fn try_map_mut<I: ?Sized, U: ?Sized>(
        mut_ref: Self::Mut<'_, I>,
        f: impl FnOnce(&mut I) -> Option<&mut U>,
    ) -> Option<Self::Mut<'_, U>> {
        mut_ref.try_map(|inner| RefMut::filter_map(inner, f).ok())
    }

    fn data_ptr(&self) -> *const () {
        self.data.as_ptr() as *const ()
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
            // If this is a rc, just ignore the drop
            RefCellStorageEntryData::Rc(_) => {}
            // If this is a reference, decrement the reference count
            RefCellStorageEntryData::Reference(reference) => {
                let reference = *reference;
                drop(borrow_mut);
                drop_ref(reference);
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

    if let RefCellStorageEntryData::Rc(entry) = &mut borrow_mut.data {
        // Decrement the reference count
        if entry.drop_ref() {
            // If the reference count is now zero, drop the value
            borrow_mut.data = RefCellStorageEntryData::Empty;
            UNSYNC_RUNTIME.with(|runtime| runtime.borrow_mut().push(pointer.storage));
        }
    } else {
        unreachable!("References should always point to a data entry directly",);
    }
}

impl<T: 'static> Storage<T> for UnsyncStorage {
    #[track_caller]
    fn try_read(
        pointer: GenerationalPointer<Self>,
    ) -> Result<Self::Ref<'static, T>, error::BorrowError> {
        let (read, pointer) = Self::read(pointer)?;

        let ref_ = Ref::filter_map(read, |any| {
            // Then try to downcast
            any.downcast_ref()
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
        let (write, pointer) = Self::write(pointer)?;

        let ref_mut = RefMut::filter_map(write, |any| {
            // Then try to downcast
            any.downcast_mut()
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

    fn new(value: T, caller: &'static std::panic::Location<'static>) -> GenerationalPointer<Self> {
        Self::create_new(RefCellStorageEntryData::Data(Box::new(value)), caller)
    }

    fn new_rc(
        value: T,
        caller: &'static std::panic::Location<'static>,
    ) -> GenerationalPointer<Self> {
        // Create the data that the rc points to
        let data = Self::create_new(
            RefCellStorageEntryData::Rc(RcStorageEntry::new(Box::new(value))),
            caller,
        );
        Self::create_new(RefCellStorageEntryData::Reference(data), caller)
    }

    fn new_reference(
        pointer: GenerationalPointer<Self>,
    ) -> BorrowResult<GenerationalPointer<Self>> {
        // Chase the reference to get the final location
        let (pointer, value) = Self::get_split_ref(pointer)?;
        if let RefCellStorageEntryData::Rc(data) = &value.data {
            data.add_ref();
        } else {
            unreachable!()
        }
        Ok(Self::create_new(
            RefCellStorageEntryData::Reference(pointer),
            pointer
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

        let mut write = location.storage.data.borrow_mut();
        // First check if the generation is still valid
        if !write.valid(&location.location) {
            return Err(BorrowError::Dropped(ValueDroppedError::new_for_location(
                location.location,
            )));
        }

        if let (RefCellStorageEntryData::Reference(reference), RefCellStorageEntryData::Rc(data)) =
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

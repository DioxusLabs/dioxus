use crate::{
    entry::{MemoryLocationBorrowInfo, StorageEntry},
    error,
    references::{GenerationalRef, GenerationalRefMut},
    AnyStorage, BorrowError, BorrowMutError, GenerationalLocation, GenerationalPointer, Storage,
};
use std::cell::{Ref, RefCell, RefMut};

thread_local! {
    static UNSYNC_RUNTIME: RefCell<Vec<&'static UnsyncStorage>> = const { RefCell::new(Vec::new()) };
}

/// A unsync storage. This is the default storage type.
#[derive(Default)]
pub struct UnsyncStorage {
    borrow_info: MemoryLocationBorrowInfo,
    data: RefCell<StorageEntry<Box<dyn std::any::Any>>>,
}

impl UnsyncStorage {
    fn try_borrow_mut(
        &self,
    ) -> Result<RefMut<'_, StorageEntry<Box<dyn std::any::Any>>>, BorrowMutError> {
        self.data
            .try_borrow_mut()
            .map_err(|_| self.borrow_info.borrow_mut_error())
    }

    fn try_borrow(&self) -> Result<Ref<'_, StorageEntry<Box<dyn std::any::Any>>>, BorrowError> {
        self.data
            .try_borrow()
            .map_err(|_| self.borrow_info.borrow_error())
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
                let location = GenerationalLocation {
                    generation: storage.data.borrow().generation(),
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    created_at: caller,
                };
                GenerationalPointer { storage, location }
            } else {
                let data: &'static Self = &*Box::leak(Box::default());
                let location = GenerationalLocation {
                    generation: 0,
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

    fn recycle(pointer: GenerationalPointer<Self>) -> Option<Box<dyn std::any::Any>> {
        let mut borrow_mut = pointer.storage.data.borrow_mut();

        // First check if the generation is still valid
        if !borrow_mut.valid(&pointer.location) {
            return None;
        }

        borrow_mut.increment_generation();
        let old_data = borrow_mut.data.take();
        UNSYNC_RUNTIME.with(|runtime| runtime.borrow_mut().push(pointer.storage));

        old_data
    }
}

impl<T: 'static> Storage<T> for UnsyncStorage {
    #[track_caller]
    fn try_read(
        pointer: GenerationalPointer<Self>,
    ) -> Result<Self::Ref<'static, T>, error::BorrowError> {
        let read = pointer.storage.try_borrow()?;

        let ref_ = Ref::filter_map(read, |any| {
            // Verify the generation is still correct
            if !any.valid(&pointer.location) {
                return None;
            }
            // Then try to downcast
            any.data.as_ref()?.downcast_ref()
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
        let write = pointer.storage.try_borrow_mut()?;

        let ref_mut = RefMut::filter_map(write, |any| {
            // Verify the generation is still correct
            if !any.valid(&pointer.location) {
                return None;
            }
            // Then try to downcast
            any.data.as_mut()?.downcast_mut()
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
        borrow_mut.data = Some(Box::new(value));
    }
}

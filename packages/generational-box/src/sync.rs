use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::sync::{Arc, OnceLock};

use crate::{
    entry::{MemoryLocationBorrowInfo, StorageEntry},
    error::{self, ValueDroppedError},
    references::{GenerationalRef, GenerationalRefMut},
    AnyStorage, GenerationalLocation, GenerationalPointer, Storage,
};

/// A thread safe storage. This is slower than the unsync storage, but allows you to share the value between threads.
#[derive(Default)]
pub struct SyncStorage {
    borrow_info: MemoryLocationBorrowInfo,
    data: RwLock<StorageEntry<Box<dyn std::any::Any + Send + Sync>>>,
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
                    generation: 0,
                    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                    created_at: caller,
                };

                GenerationalPointer { storage, location }
            }
        }
    }

    fn recycle(pointer: GenerationalPointer<Self>) -> Option<Box<dyn std::any::Any>> {
        let mut borrow_mut = pointer.storage.data.write();
        // First check if the generation is still valid
        if !borrow_mut.valid(&pointer.location) {
            return None;
        }
        borrow_mut.increment_generation();
        let old_data = borrow_mut.data.take();
        sync_runtime().lock().push(pointer.storage);
        old_data.map(|data| data as Box<dyn std::any::Any>)
    }
}

impl<T: Sync + Send + 'static> Storage<T> for SyncStorage {
    #[track_caller]
    fn try_read(
        pointer: GenerationalPointer<Self>,
    ) -> Result<Self::Ref<'static, T>, error::BorrowError> {
        let read = pointer.storage.data.read();

        let read = RwLockReadGuard::try_map(read, |any| {
            // Verify the generation is still correct
            if !any.valid(&pointer.location) {
                return None;
            }
            // Then try to downcast
            any.data.as_ref()?.downcast_ref()
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
        let write = pointer.storage.data.write();

        let write = RwLockWriteGuard::try_map(write, |any| {
            // Verify the generation is still correct
            if !any.valid(&pointer.location) {
                return None;
            }
            // Then try to downcast
            any.data.as_mut()?.downcast_mut()
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
        write.data = Some(Box::new(value));
    }
}

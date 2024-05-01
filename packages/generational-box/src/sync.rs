use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::sync::{Arc, OnceLock};

use crate::{
    error::{self, ValueDroppedError},
    references::{GenerationalRef, GenerationalRefMut},
    AnyStorage, MemoryLocation, MemoryLocationInner, Storage,
};

/// A thread safe storage. This is slower than the unsync storage, but allows you to share the value between threads.
#[derive(Default)]
pub struct SyncStorage(RwLock<Option<Box<dyn std::any::Any + Send + Sync>>>);

static SYNC_RUNTIME: OnceLock<Arc<Mutex<Vec<MemoryLocation<SyncStorage>>>>> = OnceLock::new();

fn sync_runtime() -> &'static Arc<Mutex<Vec<MemoryLocation<SyncStorage>>>> {
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

    fn try_map<I: ?Sized + 'static, U: ?Sized + 'static>(
        ref_: Self::Ref<'_, I>,
        f: impl FnOnce(&I) -> Option<&U>,
    ) -> Option<Self::Ref<'_, U>> {
        let GenerationalRef {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
            ..
        } = ref_;
        MappedRwLockReadGuard::try_map(inner, f)
            .ok()
            .map(|inner| GenerationalRef {
                inner,
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrow: crate::GenerationalRefBorrowInfo {
                    borrowed_at: borrow.borrowed_at,
                    borrowed_from: borrow.borrowed_from,
                    created_at: borrow.created_at,
                },
            })
    }

    fn try_map_mut<I: ?Sized + 'static, U: ?Sized + 'static>(
        mut_ref: Self::Mut<'_, I>,
        f: impl FnOnce(&mut I) -> Option<&mut U>,
    ) -> Option<Self::Mut<'_, U>> {
        let GenerationalRefMut {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
            ..
        } = mut_ref;
        MappedRwLockWriteGuard::try_map(inner, f)
            .ok()
            .map(|inner| GenerationalRefMut {
                inner,
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrow,
            })
    }

    fn data_ptr(&self) -> *const () {
        self.0.data_ptr() as *const ()
    }

    fn manually_drop(&self) -> bool {
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
                    borrow: Default::default(),
                }));
            MemoryLocation(data)
        })
    }

    fn recycle(location: &MemoryLocation<Self>) {
        let location = *location;
        location.drop();
        sync_runtime().lock().push(location);
    }
}

impl<T: Sync + Send + 'static> Storage<T> for SyncStorage {
    fn try_read(
        &'static self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        at: crate::GenerationalRefBorrowInfo,
    ) -> Result<Self::Ref<'static, T>, error::BorrowError> {
        let read = self.0.try_read();

        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        let read = read.ok_or_else(|| at.borrowed_from.borrow_error())?;

        #[cfg(not(any(debug_assertions, feature = "debug_ownership")))]
        let read = read.ok_or_else(|| {
            error::BorrowError::AlreadyBorrowedMut(error::AlreadyBorrowedMutError {})
        })?;

        RwLockReadGuard::try_map(read, |any| any.as_ref()?.downcast_ref())
            .map_err(|_| {
                error::BorrowError::Dropped(ValueDroppedError {
                    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                    created_at: at.created_at,
                })
            })
            .map(|guard| {
                GenerationalRef::new(
                    guard,
                    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                    at,
                )
            })
    }

    fn try_write(
        &'static self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        at: crate::GenerationalRefMutBorrowInfo,
    ) -> Result<Self::Mut<'static, T>, error::BorrowMutError> {
        let write = self.0.write();

        match RwLockWriteGuard::try_map(write, |any| any.as_mut()?.downcast_mut()) {
            Ok(guard) => Ok(GenerationalRefMut::new(
                guard,
                #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                at,
            )),
            Err(_) => Err(error::BorrowMutError::Dropped(ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                created_at: at.created_at,
            })),
        }
    }

    fn set(&self, value: T) {
        *self.0.write() = Some(Box::new(value));
    }

    fn take(&'static self) -> Option<T> {
        self.0
            .write()
            .take()
            .and_then(|any| any.downcast().ok().map(|boxed| *boxed))
    }
}

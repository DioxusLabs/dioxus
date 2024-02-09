use crate::{
    error,
    references::{GenerationalRef, GenerationalRefMut},
    AnyStorage, MemoryLocation, MemoryLocationInner, Storage,
};
use std::cell::{Ref, RefCell, RefMut};

/// A unsync storage. This is the default storage type.
#[derive(Default)]
pub struct UnsyncStorage(RefCell<Option<Box<dyn std::any::Any>>>);

impl<T: 'static> Storage<T> for UnsyncStorage {
    fn try_read(
        &'static self,

        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        at: crate::GenerationalRefBorrowInfo,
    ) -> Result<Self::Ref<T>, error::BorrowError> {
        let borrow = self.0.try_borrow();

        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        let borrow = borrow.map_err(|_| at.borrowed_from.borrow_error())?;

        #[cfg(not(any(debug_assertions, feature = "debug_ownership")))]
        let borrow = borrow.map_err(|_| {
            error::BorrowError::AlreadyBorrowedMut(error::AlreadyBorrowedMutError {})
        })?;

        Ref::filter_map(borrow, |any| any.as_ref()?.downcast_ref())
            .map_err(|_| {
                error::BorrowError::Dropped(error::ValueDroppedError {
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
    ) -> Result<Self::Mut<T>, error::BorrowMutError> {
        let borrow = self.0.try_borrow_mut();

        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        let borrow = borrow.map_err(|_| at.borrowed_from.borrow_mut_error())?;

        #[cfg(not(any(debug_assertions, feature = "debug_ownership")))]
        let borrow = borrow
            .map_err(|_| error::BorrowMutError::AlreadyBorrowed(error::AlreadyBorrowedError {}))?;

        RefMut::filter_map(borrow, |any| any.as_mut()?.downcast_mut())
            .map_err(|_| {
                error::BorrowMutError::Dropped(error::ValueDroppedError {
                    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                    created_at: at.created_at,
                })
            })
            .map(|guard| {
                GenerationalRefMut::new(
                    guard,
                    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                    at,
                )
            })
    }

    fn set(&self, value: T) {
        *self.0.borrow_mut() = Some(Box::new(value));
    }

    fn take(&'static self) -> Option<T> {
        self.0
            .borrow_mut()
            .take()
            .map(|any| *any.downcast().unwrap())
    }
}

thread_local! {
    static UNSYNC_RUNTIME: RefCell<Vec<MemoryLocation<UnsyncStorage>>> = const { RefCell::new(Vec::new()) };
}

impl AnyStorage for UnsyncStorage {
    type Ref<R: ?Sized + 'static> = GenerationalRef<Ref<'static, R>>;
    type Mut<W: ?Sized + 'static> = GenerationalRefMut<RefMut<'static, W>>;

    fn try_map<I: ?Sized, U: ?Sized + 'static>(
        _self: Self::Ref<I>,
        f: impl FnOnce(&I) -> Option<&U>,
    ) -> Option<Self::Ref<U>> {
        let GenerationalRef {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
            ..
        } = _self;
        Ref::filter_map(inner, f).ok().map(|inner| GenerationalRef {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
        })
    }

    fn try_map_mut<I: ?Sized, U: ?Sized + 'static>(
        mut_ref: Self::Mut<I>,
        f: impl FnOnce(&mut I) -> Option<&mut U>,
    ) -> Option<Self::Mut<U>> {
        let GenerationalRefMut {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
            ..
        } = mut_ref;
        RefMut::filter_map(inner, f)
            .ok()
            .map(|inner| GenerationalRefMut {
                inner,
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrow: crate::GenerationalRefMutBorrowInfo {
                    borrowed_from: borrow.borrowed_from,
                    created_at: borrow.created_at,
                },
            })
    }

    fn data_ptr(&self) -> *const () {
        self.0.as_ptr() as *const ()
    }

    fn manually_drop(&self) -> bool {
        self.0.borrow_mut().take().is_some()
    }

    fn claim() -> MemoryLocation<Self> {
        UNSYNC_RUNTIME.with(|runtime| {
            if let Some(location) = runtime.borrow_mut().pop() {
                location
            } else {
                let data: &'static MemoryLocationInner =
                    &*Box::leak(Box::new(MemoryLocationInner {
                        data: Self::default(),
                        #[cfg(any(debug_assertions, feature = "check_generation"))]
                        generation: 0.into(),
                        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                        borrow: Default::default(),
                    }));
                MemoryLocation(data)
            }
        })
    }

    fn recycle(location: &MemoryLocation<Self>) {
        location.drop();
        UNSYNC_RUNTIME.with(|runtime| runtime.borrow_mut().push(*location));
    }
}

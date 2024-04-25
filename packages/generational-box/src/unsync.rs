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
    ) -> Result<Self::Ref<'static, T>, error::BorrowError> {
        let borrow = self.0.try_borrow();

        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        let borrow = borrow.map_err(|_| at.borrowed_from.borrow_error())?;

        #[cfg(not(any(debug_assertions, feature = "debug_ownership")))]
        let borrow = borrow.map_err(|_| {
            error::BorrowError::AlreadyBorrowedMut(error::AlreadyBorrowedMutError {})
        })?;

        match Ref::filter_map(borrow, |any| any.as_ref()?.downcast_ref()) {
            Ok(guard) => Ok(GenerationalRef::new(
                guard,
                #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                at,
            )),
            Err(_) => Err(error::BorrowError::Dropped(error::ValueDroppedError {
                #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                created_at: at.created_at,
            })),
        }
    }

    fn try_write(
        &'static self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        at: crate::GenerationalRefMutBorrowInfo,
    ) -> Result<Self::Mut<'static, T>, error::BorrowMutError> {
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

    fn try_map<I: ?Sized + 'static, U: ?Sized + 'static>(
        _self: Self::Ref<'_, I>,
        f: impl FnOnce(&I) -> Option<&U>,
    ) -> Option<Self::Ref<'_, U>> {
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
        RefMut::filter_map(inner, f)
            .ok()
            .map(|inner| GenerationalRefMut {
                inner,
                #[cfg(any(debug_assertions, feature = "debug_borrows"))]
                borrow,
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
        let location = *location;
        location.drop();
        UNSYNC_RUNTIME.with(|runtime| runtime.borrow_mut().push(location));
    }
}

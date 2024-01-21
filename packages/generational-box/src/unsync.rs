use crate::{
    references::{GenerationalRef, GenerationalRefMut},
    AnyStorage, Mappable, MappableMut, MemoryLocation, MemoryLocationInner, Storage,
};
use std::cell::{Ref, RefCell, RefMut};

/// A unsync storage. This is the default storage type.
pub struct UnsyncStorage(RefCell<Option<Box<dyn std::any::Any>>>);

impl Default for UnsyncStorage {
    fn default() -> Self {
        Self(RefCell::new(None))
    }
}

impl<T: ?Sized> Mappable<T> for Ref<'static, T> {
    type Mapped<U: ?Sized + 'static> = Ref<'static, U>;

    fn map<U: ?Sized + 'static>(_self: Self, f: impl FnOnce(&T) -> &U) -> Self::Mapped<U> {
        Ref::map(_self, f)
    }

    fn try_map<U: ?Sized + 'static>(
        _self: Self,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<Self::Mapped<U>> {
        Ref::filter_map(_self, f).ok()
    }
}

impl<T: ?Sized> MappableMut<T> for RefMut<'static, T> {
    type Mapped<U: ?Sized + 'static> = RefMut<'static, U>;

    fn map<U: ?Sized + 'static>(_self: Self, f: impl FnOnce(&mut T) -> &mut U) -> Self::Mapped<U> {
        RefMut::map(_self, f)
    }

    fn try_map<U: ?Sized + 'static>(
        _self: Self,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Self::Mapped<U>> {
        RefMut::filter_map(_self, f).ok()
    }
}

impl<T: 'static> Storage<T> for UnsyncStorage {
    type Ref = GenerationalRef<T, Ref<'static, T>>;
    type Mut = GenerationalRefMut<T, RefMut<'static, T>>;

    fn try_read(
        &'static self,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        created_at: &'static std::panic::Location<'static>,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        at: crate::GenerationalRefBorrowInfo,
    ) -> Result<Self::Ref, crate::error::BorrowError> {
        let borrow = self.0.try_borrow();

        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        let borrow = borrow.map_err(|_| at.borrowed_from.borrow_error())?;

        #[cfg(not(any(debug_assertions, feature = "debug_ownership")))]
        let borrow = borrow.unwrap();

        Ref::filter_map(borrow, |any| any.as_ref()?.downcast_ref())
            .map_err(|_| {
                crate::error::BorrowError::Dropped(crate::error::ValueDroppedError {
                    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                    created_at,
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
        created_at: &'static std::panic::Location<'static>,
        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        at: crate::GenerationalRefMutBorrowInfo,
    ) -> Result<Self::Mut, crate::error::BorrowMutError> {
        let borrow = self.0.try_borrow_mut();

        #[cfg(any(debug_assertions, feature = "debug_ownership"))]
        let borrow = borrow.map_err(|_| at.borrowed_from.borrow_mut_error())?;

        #[cfg(not(any(debug_assertions, feature = "debug_ownership")))]
        let borrow = borrow.unwrap();

        RefMut::filter_map(borrow, |any| any.as_mut()?.downcast_mut())
            .map_err(|_| {
                crate::error::BorrowMutError::Dropped(crate::error::ValueDroppedError {
                    #[cfg(any(debug_assertions, feature = "debug_ownership"))]
                    created_at,
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
}

thread_local! {
    static UNSYNC_RUNTIME: RefCell<Vec<MemoryLocation<UnsyncStorage>>> = RefCell::new(Vec::new());
}

impl AnyStorage for UnsyncStorage {
    fn data_ptr(&self) -> *const () {
        self.0.as_ptr() as *const ()
    }

    fn take(&self) -> bool {
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

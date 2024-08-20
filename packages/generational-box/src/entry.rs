use crate::{
    BorrowError, BorrowMutError, GenerationalLocation, GenerationalRefBorrowGuard,
    GenerationalRefBorrowMutGuard,
};
use std::num::NonZeroU64;

pub(crate) struct FullStorageEntry<T> {
    ref_count: NonZeroU64,
    pub data: T,
}

impl<T> FullStorageEntry<T> {
    pub const fn new(data: T) -> Self {
        Self {
            ref_count: NonZeroU64::MIN,
            data,
        }
    }

    pub fn add_ref(&mut self) {
        self.ref_count = self.ref_count.checked_add(1).unwrap();
    }

    pub fn drop_ref(&mut self) -> bool {
        let ref_count = self.ref_count.get();
        match NonZeroU64::new(ref_count - 1) {
            Some(ref_count) => {
                self.ref_count = ref_count;
                false
            }
            None => true,
        }
    }
}

pub(crate) struct StorageEntry<T> {
    generation: NonZeroU64,
    pub(crate) data: T,
}

impl<T> StorageEntry<T> {
    pub const fn new(data: T) -> Self {
        Self {
            generation: NonZeroU64::MIN,
            data,
        }
    }

    pub fn valid(&self, location: &GenerationalLocation) -> bool {
        self.generation == location.generation
    }

    pub fn increment_generation(&mut self) {
        self.generation = self.generation.checked_add(1).unwrap();
    }

    pub fn generation(&self) -> NonZeroU64 {
        self.generation
    }
}

impl<T: Default + 'static> Default for StorageEntry<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[derive(Default)]
pub(crate) struct MemoryLocationBorrowInfo(
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    parking_lot::RwLock<MemoryLocationBorrowInfoInner>,
);

impl MemoryLocationBorrowInfo {
    pub(crate) fn borrow_mut_error(&self) -> BorrowMutError {
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        {
            let borrow = self.0.read();
            if let Some(borrowed_mut_at) = borrow.borrowed_mut_at.as_ref() {
                BorrowMutError::AlreadyBorrowedMut(crate::error::AlreadyBorrowedMutError {
                    borrowed_mut_at,
                })
            } else {
                BorrowMutError::AlreadyBorrowed(crate::error::AlreadyBorrowedError {
                    borrowed_at: borrow.borrowed_at.clone(),
                })
            }
        }
        #[cfg(not(any(debug_assertions, feature = "debug_borrows")))]
        {
            BorrowMutError::AlreadyBorrowed(crate::error::AlreadyBorrowedError {})
        }
    }

    pub(crate) fn borrow_error(&self) -> BorrowError {
        BorrowError::AlreadyBorrowedMut(crate::error::AlreadyBorrowedMutError {
            #[cfg(any(debug_assertions, feature = "debug_ownership"))]
            borrowed_mut_at: self.0.read().borrowed_mut_at.unwrap(),
        })
    }

    /// Start a new borrow
    #[track_caller]
    pub(crate) fn borrow_guard(&'static self) -> GenerationalRefBorrowGuard {
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        let borrowed_at = std::panic::Location::caller();
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        {
            let mut borrow = self.0.write();
            borrow.borrowed_at.push(borrowed_at);
        }

        GenerationalRefBorrowGuard {
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrowed_at,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrowed_from: self,
        }
    }

    /// Start a new mutable borrow
    #[track_caller]
    pub(crate) fn borrow_mut_guard(&'static self) -> GenerationalRefBorrowMutGuard {
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        let borrowed_mut_at = std::panic::Location::caller();
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        {
            let mut borrow = self.0.write();
            borrow.borrowed_mut_at = Some(borrowed_mut_at);
        }

        GenerationalRefBorrowMutGuard {
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrowed_mut_at,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrowed_from: self,
        }
    }

    #[allow(unused)]
    pub(crate) fn drop_borrow(&self, borrowed_at: &'static std::panic::Location<'static>) {
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        {
            let mut borrow = self.0.write();
            borrow
                .borrowed_at
                .retain(|location| *location != borrowed_at);
        }
    }

    #[allow(unused)]
    pub(crate) fn drop_borrow_mut(&self, borrowed_mut_at: &'static std::panic::Location<'static>) {
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        {
            let mut borrow = self.0.write();
            if borrow.borrowed_mut_at == Some(borrowed_mut_at) {
                borrow.borrowed_mut_at = None;
            }
        }
    }
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
#[derive(Default)]
struct MemoryLocationBorrowInfoInner {
    borrowed_at: Vec<&'static std::panic::Location<'static>>,
    borrowed_mut_at: Option<&'static std::panic::Location<'static>>,
}

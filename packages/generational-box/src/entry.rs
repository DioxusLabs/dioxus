use crate::{
    BorrowError, BorrowMutError, GenerationalLocation, GenerationalRefBorrowGuard,
    GenerationalRefBorrowMutGuard,
};

pub(crate) struct StorageEntry<T> {
    generation: u64,

    pub data: Option<T>,
}

impl<T> Default for StorageEntry<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            data: None,
        }
    }
}

impl<T> StorageEntry<T> {
    pub fn valid(&self, location: &GenerationalLocation) -> bool {
        self.generation == location.generation
    }

    pub fn increment_generation(&mut self) {
        self.generation += 1;
    }

    pub fn generation(&self) -> u64 {
        self.generation
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

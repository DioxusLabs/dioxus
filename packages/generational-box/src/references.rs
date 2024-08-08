use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

/// A reference to a value in a generational box.
pub struct GenerationalRef<R> {
    pub(crate) inner: R,
    guard: GenerationalRefBorrowGuard,
}

impl<T: ?Sized + 'static, R: Deref<Target = T>> GenerationalRef<R> {
    pub(crate) fn new(inner: R, guard: GenerationalRefBorrowGuard) -> Self {
        Self { inner, guard }
    }

    /// Map the inner value to a new type
    pub fn map<R2, F: FnOnce(R) -> R2>(self, f: F) -> GenerationalRef<R2> {
        GenerationalRef {
            inner: f(self.inner),
            guard: self.guard,
        }
    }

    /// Try to map the inner value to a new type
    pub fn try_map<R2, F: FnOnce(R) -> Option<R2>>(self, f: F) -> Option<GenerationalRef<R2>> {
        f(self.inner).map(|inner| GenerationalRef {
            inner,
            guard: self.guard,
        })
    }
}

impl<T: ?Sized + Debug, R: Deref<Target = T>> Debug for GenerationalRef<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.deref().fmt(f)
    }
}

impl<T: ?Sized + Display, R: Deref<Target = T>> Display for GenerationalRef<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.deref().fmt(f)
    }
}

impl<T: ?Sized + 'static, R: Deref<Target = T>> Deref for GenerationalRef<R> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

pub(crate) struct GenerationalRefBorrowGuard {
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    pub(crate) borrowed_at: &'static std::panic::Location<'static>,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    pub(crate) borrowed_from: &'static crate::entry::MemoryLocationBorrowInfo,
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
impl Drop for GenerationalRefBorrowGuard {
    fn drop(&mut self) {
        self.borrowed_from.drop_borrow(self.borrowed_at);
    }
}

/// A mutable reference to a value in a generational box.
pub struct GenerationalRefMut<W> {
    pub(crate) inner: W,
    pub(crate) borrow: GenerationalRefBorrowMutGuard,
}

impl<T: ?Sized + 'static, R: DerefMut<Target = T>> GenerationalRefMut<R> {
    pub(crate) fn new(inner: R, borrow: GenerationalRefBorrowMutGuard) -> Self {
        Self { inner, borrow }
    }

    /// Map the inner value to a new type
    pub fn map<R2, F: FnOnce(R) -> R2>(self, f: F) -> GenerationalRefMut<R2> {
        GenerationalRefMut {
            inner: f(self.inner),
            borrow: self.borrow,
        }
    }

    /// Try to map the inner value to a new type
    pub fn try_map<R2, F: FnOnce(R) -> Option<R2>>(self, f: F) -> Option<GenerationalRefMut<R2>> {
        f(self.inner).map(|inner| GenerationalRefMut {
            inner,
            borrow: self.borrow,
        })
    }
}

impl<T: ?Sized, W: DerefMut<Target = T>> Deref for GenerationalRefMut<W> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: ?Sized, W: DerefMut<Target = T>> DerefMut for GenerationalRefMut<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

pub(crate) struct GenerationalRefBorrowMutGuard {
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    /// The location where the borrow occurred.
    pub(crate) borrowed_from: &'static crate::entry::MemoryLocationBorrowInfo,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    pub(crate) borrowed_mut_at: &'static std::panic::Location<'static>,
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
impl Drop for GenerationalRefBorrowMutGuard {
    fn drop(&mut self) {
        self.borrowed_from.drop_borrow_mut(self.borrowed_mut_at);
    }
}

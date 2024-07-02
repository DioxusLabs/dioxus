use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

/// A reference to a value in a generational box.
pub struct GenerationalRef<R> {
    pub(crate) inner: R,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    pub(crate) borrow: GenerationalRefBorrowInfo,
}

impl<T: 'static, R: Deref<Target = T>> GenerationalRef<R> {
    pub(crate) fn new(
        inner: R,
        #[cfg(any(debug_assertions, feature = "debug_borrows"))] borrow: GenerationalRefBorrowInfo,
    ) -> Self {
        Self {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
        }
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

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
/// Information about a borrow.
pub struct GenerationalRefBorrowInfo {
    pub(crate) borrowed_at: &'static std::panic::Location<'static>,
    pub(crate) borrowed_from: &'static crate::MemoryLocationBorrowInfo,
    pub(crate) created_at: &'static std::panic::Location<'static>,
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
impl Drop for GenerationalRefBorrowInfo {
    fn drop(&mut self) {
        self.borrowed_from
            .borrowed_at
            .write()
            .retain(|location| !std::ptr::eq(*location, self.borrowed_at as *const _));
    }
}

/// A mutable reference to a value in a generational box.
pub struct GenerationalRefMut<W> {
    pub(crate) inner: W,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    pub(crate) borrow: GenerationalRefBorrowMutGuard,
}

impl<T, R: DerefMut<Target = T>> GenerationalRefMut<R> {
    pub(crate) fn new(
        inner: R,
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        borrow: GenerationalRefMutBorrowInfo,
    ) -> Self {
        Self {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow: borrow.into(),
        }
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

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
/// Information about a mutable borrow.
pub struct GenerationalRefMutBorrowInfo {
    /// The location where the borrow occurred.
    pub(crate) borrowed_from: &'static crate::MemoryLocationBorrowInfo,
    pub(crate) created_at: &'static std::panic::Location<'static>,
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
pub(crate) struct GenerationalRefBorrowMutGuard {
    borrow_info: GenerationalRefMutBorrowInfo,
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
impl From<GenerationalRefMutBorrowInfo> for GenerationalRefBorrowMutGuard {
    fn from(borrow_info: GenerationalRefMutBorrowInfo) -> Self {
        Self { borrow_info }
    }
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
impl Drop for GenerationalRefBorrowMutGuard {
    fn drop(&mut self) {
        self.borrow_info
            .borrowed_from
            .borrowed_mut_at
            .write()
            .take();
    }
}

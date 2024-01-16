use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{Mappable, MappableMut};

/// A reference to a value in a generational box.
pub struct GenerationalRef<T: ?Sized + 'static, R: Mappable<T>> {
    inner: R,
    phantom: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrow: GenerationalRefBorrowInfo,
}

impl<T: 'static, R: Mappable<T>> GenerationalRef<T, R> {
    pub(crate) fn new(
        inner: R,
        #[cfg(any(debug_assertions, feature = "debug_borrows"))] borrow: GenerationalRefBorrowInfo,
    ) -> Self {
        Self {
            inner,
            phantom: PhantomData,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
        }
    }
}

impl<T: ?Sized + 'static, R: Mappable<T>> Mappable<T> for GenerationalRef<T, R> {
    type Mapped<U: ?Sized + 'static> = GenerationalRef<U, R::Mapped<U>>;

    fn map<U: ?Sized + 'static>(_self: Self, f: impl FnOnce(&T) -> &U) -> Self::Mapped<U> {
        GenerationalRef {
            inner: R::map(_self.inner, f),
            phantom: PhantomData,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow: GenerationalRefBorrowInfo {
                borrowed_at: _self.borrow.borrowed_at,
                borrowed_from: _self.borrow.borrowed_from,
            },
        }
    }

    fn try_map<U: ?Sized + 'static>(
        _self: Self,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<Self::Mapped<U>> {
        let Self {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
            ..
        } = _self;
        R::try_map(inner, f).map(|inner| GenerationalRef {
            inner,
            phantom: PhantomData,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow: GenerationalRefBorrowInfo {
                borrowed_at: borrow.borrowed_at,
                borrowed_from: borrow.borrowed_from,
            },
        })
    }
}

impl<T: ?Sized + Debug, R: Mappable<T>> Debug for GenerationalRef<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.deref().fmt(f)
    }
}

impl<T: ?Sized + Display, R: Mappable<T>> Display for GenerationalRef<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.deref().fmt(f)
    }
}

impl<T: ?Sized + 'static, R: Mappable<T>> Deref for GenerationalRef<T, R> {
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
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
impl Drop for GenerationalRefBorrowInfo {
    fn drop(&mut self) {
        self.borrowed_from
            .borrowed_at
            .write()
            .retain(|location| std::ptr::eq(*location, self.borrowed_at as *const _));
    }
}

/// A mutable reference to a value in a generational box.
pub struct GenerationalRefMut<T: ?Sized + 'static, W: MappableMut<T>> {
    inner: W,
    phantom: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "debug_borrows"))]
    borrow: GenerationalRefMutBorrowInfo,
}

impl<T: 'static, R: MappableMut<T>> GenerationalRefMut<T, R> {
    pub(crate) fn new(
        inner: R,
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        borrow: GenerationalRefMutBorrowInfo,
    ) -> Self {
        Self {
            inner,
            phantom: PhantomData,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
        }
    }
}

impl<T: ?Sized + 'static, W: MappableMut<T>> MappableMut<T> for GenerationalRefMut<T, W> {
    type Mapped<U: ?Sized + 'static> = GenerationalRefMut<U, W::Mapped<U>>;

    fn map<U: ?Sized + 'static>(_self: Self, f: impl FnOnce(&mut T) -> &mut U) -> Self::Mapped<U> {
        GenerationalRefMut {
            inner: W::map(_self.inner, f),
            phantom: PhantomData,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow: _self.borrow,
        }
    }

    fn try_map<U: ?Sized + 'static>(
        _self: Self,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Self::Mapped<U>> {
        let Self {
            inner,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
            ..
        } = _self;
        W::try_map(inner, f).map(|inner| GenerationalRefMut {
            inner,
            phantom: PhantomData,
            #[cfg(any(debug_assertions, feature = "debug_borrows"))]
            borrow,
        })
    }
}

impl<T: ?Sized + 'static, W: MappableMut<T>> Deref for GenerationalRefMut<T, W> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: ?Sized + 'static, W: MappableMut<T>> DerefMut for GenerationalRefMut<T, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
/// Information about a mutable borrow.
pub struct GenerationalRefMutBorrowInfo {
    /// The location where the borrow occurred.
    pub(crate) borrowed_from: &'static crate::MemoryLocationBorrowInfo,
}

#[cfg(any(debug_assertions, feature = "debug_borrows"))]
impl Drop for GenerationalRefMutBorrowInfo {
    fn drop(&mut self) {
        self.borrowed_from.borrowed_mut_at.write().take();
    }
}

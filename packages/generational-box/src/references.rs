use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

/// A reference to a value in a generational box. This reference acts similarly to [`std::cell::Ref`], but has extra debug information
/// to track when all references to the value are created and dropped.
///
/// [`GenerationalRef`] implements [`Deref`] which means you can call methods on the inner value just like you would on a reference to the
/// inner value. If you need to get the inner reference directly, you can call [`GenerationalRef::deref`].
///
/// # Example
/// ```rust
/// # use generational_box::{Owner, UnsyncStorage, AnyStorage};
/// let owner = UnsyncStorage::owner();
/// let value = owner.insert(String::from("hello"));
/// let reference = value.read();
///
/// // You call methods like `as_str` on the reference just like you would with the inner String
/// assert_eq!(reference.as_str(), "hello");
/// ```
///
/// ## Matching on GenerationalRef
///
/// You need to get the inner reference with [`GenerationalRef::deref`] before you match the inner value. If you try to match without
/// calling [`GenerationalRef::deref`], you will get an error like this:
///
/// ```compile_fail
/// # use generational_box::{Owner, UnsyncStorage, AnyStorage};
/// enum Colors {
///     Red,
///     Green
/// }
/// let owner = UnsyncStorage::owner();
/// let value = owner.insert(Colors::Red);
/// let reference = value.read();
///
/// match reference {
///     // Since we are matching on the `GenerationalRef` type instead of &Colors, we can't match on the enum directly
///     Colors::Red => {}
///     Colors::Green => {}
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> packages/generational-box/tests/basic.rs:25:9
///   |
/// 2 |         Red,
///   |         --- unit variant defined here
/// ...
/// 3 |     match reference {
///   |           --------- this expression has type `GenerationalRef<Ref<'_, Colors>>`
/// 4 |         // Since we are matching on the `GenerationalRef` type instead of &Colors, we can't match on the enum directly
/// 5 |         Colors::Red => {}
///   |         ^^^^^^^^^^^ expected `GenerationalRef<Ref<'_, Colors>>`, found `Colors`
///   |
///   = note: expected struct `GenerationalRef<Ref<'_, Colors>>`
///                found enum `Colors`
/// ```
///
/// Instead, you need to deref the reference to get the inner value **before** you match on it:
///
/// ```rust
/// use std::ops::Deref;
/// # use generational_box::{AnyStorage, Owner, UnsyncStorage};
/// enum Colors {
///     Red,
///     Green
/// }
/// let owner = UnsyncStorage::owner();
/// let value = owner.insert(Colors::Red);
/// let reference = value.read();
///
/// // Deref converts the `GenerationalRef` into a `&Colors`
/// match reference.deref() {
///     // Now we can match on the inner value
///     Colors::Red => {}
///     Colors::Green => {}
/// }
/// ```
pub struct GenerationalRef<R> {
    pub(crate) inner: R,
    guard: GenerationalRefBorrowGuard,
}

impl<T: ?Sized, R: Deref<Target = T>> GenerationalRef<R> {
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

    /// Clone the inner value. This requires that the inner value implements [`Clone`].
    pub fn cloned(&self) -> T
    where
        T: Clone,
    {
        self.inner.deref().clone()
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

impl<T: ?Sized, R: Deref<Target = T>> Deref for GenerationalRef<R> {
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

/// A mutable reference to a value in a generational box. This reference acts similarly to [`std::cell::RefMut`], but has extra debug information
/// to track when all references to the value are created and dropped.
///
/// [`GenerationalRefMut`] implements [`DerefMut`] which means you can call methods on the inner value just like you would on a mutable reference
/// to the inner value. If you need to get the inner reference directly, you can call [`GenerationalRefMut::deref_mut`].
///
/// # Example
/// ```rust
/// # use generational_box::{Owner, UnsyncStorage, AnyStorage};
/// let owner = UnsyncStorage::owner();
/// let mut value = owner.insert(String::from("hello"));
/// let mut mutable_reference = value.write();
///
/// // You call methods like `push_str` on the reference just like you would with the inner String
/// mutable_reference.push_str("world");
/// ```
///
/// ## Matching on GenerationalMut
///
/// You need to get the inner mutable reference with [`GenerationalRefMut::deref_mut`] before you match the inner value. If you try to match
/// without calling [`GenerationalRefMut::deref_mut`], you will get an error like this:
///
/// ```compile_fail
/// # use generational_box::{Owner, UnsyncStorage, AnyStorage};
/// enum Colors {
///     Red(u32),
///     Green
/// }
/// let owner = UnsyncStorage::owner();
/// let mut value = owner.insert(Colors::Red(0));
/// let mut mutable_reference = value.write();
///
/// match mutable_reference {
///     // Since we are matching on the `GenerationalRefMut` type instead of &mut Colors, we can't match on the enum directly
///     Colors::Red(brightness) => *brightness += 1,
///     Colors::Green => {}
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> packages/generational-box/tests/basic.rs:25:9
///    |
/// 9  |     match mutable_reference {
///    |           ----------------- this expression has type `GenerationalRefMut<RefMut<'_, fn(u32) -> Colors {Colors::Red}>>`
/// 10 |         // Since we are matching on the `GenerationalRefMut` type instead of &mut Colors, we can't match on the enum directly
/// 11 |         Colors::Red(brightness) => *brightness += 1,
///    |         ^^^^^^^^^^^^^^^^^^^^^^^ expected `GenerationalRefMut<RefMut<'_, ...>>`, found `Colors`
///    |
///    = note: expected struct `GenerationalRefMut<RefMut<'_, fn(u32) -> Colors {Colors::Red}>>`
///                found enum `Colors`
/// ```
///
/// Instead, you need to call deref mut on the reference to get the inner value **before** you match on it:
///
/// ```rust
/// use std::ops::DerefMut;
/// # use generational_box::{AnyStorage, Owner, UnsyncStorage};
/// enum Colors {
///     Red(u32),
///     Green
/// }
/// let owner = UnsyncStorage::owner();
/// let mut value = owner.insert(Colors::Red(0));
/// let mut mutable_reference = value.write();
///
/// // DerefMut converts the `GenerationalRefMut` into a `&mut Colors`
/// match mutable_reference.deref_mut() {
///     // Now we can match on the inner value
///     Colors::Red(brightness) => *brightness += 1,
///     Colors::Green => {}
/// }
/// ```
pub struct GenerationalRefMut<W> {
    pub(crate) inner: W,
    pub(crate) borrow: GenerationalRefBorrowMutGuard,
}

impl<T: ?Sized, R: DerefMut<Target = T>> GenerationalRefMut<R> {
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

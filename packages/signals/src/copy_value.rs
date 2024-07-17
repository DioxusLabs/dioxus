use generational_box::GenerationalBoxId;
use generational_box::UnsyncStorage;
use std::ops::Deref;

use dioxus_core::prelude::*;

use generational_box::{GenerationalBox, Storage};

use crate::read_impls;
use crate::Readable;
use crate::ReadableRef;
use crate::Writable;
use crate::WritableRef;
use crate::{default_impl, write_impls};

/// CopyValue is a wrapper around a value to make the value mutable and Copy.
///
/// It is internally backed by [`generational_box::GenerationalBox`].
pub struct CopyValue<T: 'static, S: Storage<T> = UnsyncStorage> {
    pub(crate) value: GenerationalBox<T, S>,
    origin_scope: ScopeId,
}

#[cfg(feature = "serialize")]
impl<T: 'static, Store: Storage<T>> serde::Serialize for CopyValue<T, Store>
where
    T: serde::Serialize,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.read().serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de, T: 'static, Store: Storage<T>> serde::Deserialize<'de> for CopyValue<T, Store>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = T::deserialize(deserializer)?;

        Ok(Self::new_maybe_sync(value))
    }
}

impl<T: 'static> CopyValue<T> {
    /// Create a new CopyValue. The value will be stored in the current component.
    ///
    /// Once the component this value is created in is dropped, the value will be dropped.
    #[track_caller]
    pub fn new(value: T) -> Self {
        Self::new_maybe_sync(value)
    }

    /// Create a new CopyValue. The value will be stored in the given scope. When the specified scope is dropped, the value will be dropped.
    #[track_caller]
    pub fn new_in_scope(value: T, scope: ScopeId) -> Self {
        Self::new_maybe_sync_in_scope(value, scope)
    }
}

impl<T: 'static, S: Storage<T>> CopyValue<T, S> {
    /// Create a new CopyValue. The value will be stored in the current component.
    ///
    /// Once the component this value is created in is dropped, the value will be dropped.
    #[track_caller]
    pub fn new_maybe_sync(value: T) -> Self {
        let owner = current_owner();

        Self {
            value: owner.insert(value),
            origin_scope: current_scope_id().expect("in a virtual dom"),
        }
    }

    pub(crate) fn new_with_caller(
        value: T,
        caller: &'static std::panic::Location<'static>,
    ) -> Self {
        let owner = current_owner();

        Self {
            value: owner.insert_with_caller(value, caller),
            origin_scope: current_scope_id().expect("in a virtual dom"),
        }
    }

    /// Create a new CopyValue. The value will be stored in the given scope. When the specified scope is dropped, the value will be dropped.
    #[track_caller]
    pub fn new_maybe_sync_in_scope(value: T, scope: ScopeId) -> Self {
        let owner = scope.owner();

        Self {
            value: owner.insert(value),
            origin_scope: scope,
        }
    }

    /// Manually drop the value in the CopyValue, invalidating the value in the process.
    pub fn manually_drop(&self) -> Option<T> {
        self.value.manually_drop()
    }

    /// Get the scope this value was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.origin_scope
    }

    /// Get the generational id of the value.
    pub fn id(&self) -> GenerationalBoxId {
        self.value.id()
    }

    /// Get the underlying [`GenerationalBox`] value.
    pub fn value(&self) -> GenerationalBox<T, S> {
        self.value
    }
}

impl<T: 'static, S: Storage<T>> Readable for CopyValue<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value.try_read()
    }

    #[track_caller]
    fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
        self.value.read()
    }
}

impl<T: 'static, S: Storage<T>> Writable for CopyValue<T, S> {
    type Mut<'a, R: ?Sized + 'static> = S::Mut<'a, R>;

    fn map_mut<I: ?Sized, U: ?Sized, F: FnOnce(&mut I) -> &mut U>(
        mut_: Self::Mut<'_, I>,
        f: F,
    ) -> Self::Mut<'_, U> {
        S::map_mut(mut_, f)
    }

    fn try_map_mut<I: ?Sized, U: ?Sized, F: FnOnce(&mut I) -> Option<&mut U>>(
        mut_: Self::Mut<'_, I>,
        f: F,
    ) -> Option<Self::Mut<'_, U>> {
        S::try_map_mut(mut_, f)
    }

    fn downcast_lifetime_mut<'a: 'b, 'b, R: ?Sized + 'static>(
        mut_: Self::Mut<'a, R>,
    ) -> Self::Mut<'b, R> {
        S::downcast_lifetime_mut(mut_)
    }

    #[track_caller]
    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError> {
        self.value.try_write()
    }

    #[track_caller]
    fn set(&mut self, value: T) {
        self.value.set(value);
    }
}

impl<T: 'static, S: Storage<T>> PartialEq for CopyValue<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.value.ptr_eq(&other.value)
    }
}
impl<T: 'static, S: Storage<T>> Eq for CopyValue<T, S> {}

impl<T: Copy, S: Storage<T>> Deref for CopyValue<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        Readable::deref_impl(self)
    }
}

impl<T, S: Storage<T>> Clone for CopyValue<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S: Storage<T>> Copy for CopyValue<T, S> {}

read_impls!(CopyValue<T, S: Storage<T>>);
default_impl!(CopyValue<T, S: Storage<T>>);
write_impls!(CopyValue<T, S: Storage<T>>);

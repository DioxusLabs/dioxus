#![allow(clippy::unnecessary_operation)]
#![allow(clippy::no_effect)]

use dioxus_core::{current_owner, current_scope_id, ScopeId};
use dioxus_core::{Runtime, Subscribers};
use generational_box::{
    AnyStorage, BorrowResult, GenerationalBox, GenerationalBoxId, Storage, UnsyncStorage,
};
use std::ops::Deref;

use crate::read_impls;
use crate::Readable;
use crate::ReadableExt;
use crate::ReadableRef;
use crate::Writable;
use crate::WritableRef;
use crate::WriteLock;
use crate::{default_impl, write_impls, WritableExt};

/// CopyValue is a wrapper around a value to make the value mutable and Copy.
///
/// It is internally backed by [`generational_box::GenerationalBox`].
pub struct CopyValue<T, S: 'static = UnsyncStorage> {
    pub(crate) value: GenerationalBox<T, S>,
    pub(crate) origin_scope: ScopeId,
}

#[cfg(feature = "serialize")]
impl<T, Store: Storage<T>> serde::Serialize for CopyValue<T, Store>
where
    T: serde::Serialize + 'static,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.read().serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de, T, Store: Storage<T>> serde::Deserialize<'de> for CopyValue<T, Store>
where
    T: serde::Deserialize<'de> + 'static,
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

impl<T, S: Storage<T>> CopyValue<T, S> {
    /// Create a new CopyValue. The value will be stored in the current component.
    ///
    /// Once the component this value is created in is dropped, the value will be dropped.
    #[track_caller]
    pub fn new_maybe_sync(value: T) -> Self
    where
        T: 'static,
    {
        Self::new_with_caller(value, std::panic::Location::caller())
    }

    /// Create a new CopyValue without an owner. This will leak memory if you don't manually drop it.
    pub fn leak_with_caller(value: T, caller: &'static std::panic::Location<'static>) -> Self
    where
        T: 'static,
    {
        Self {
            value: GenerationalBox::leak(value, caller),
            origin_scope: current_scope_id(),
        }
    }

    /// Point to another copy value
    pub fn point_to(&self, other: Self) -> BorrowResult {
        self.value.point_to(other.value)
    }

    pub(crate) fn new_with_caller(
        value: T,
        caller: &'static std::panic::Location<'static>,
    ) -> Self {
        let owner = current_owner();

        Self {
            value: owner.insert_rc_with_caller(value, caller),
            origin_scope: current_scope_id(),
        }
    }

    /// Create a new CopyValue. The value will be stored in the given scope. When the specified scope is dropped, the value will be dropped.
    #[track_caller]
    pub fn new_maybe_sync_in_scope(value: T, scope: ScopeId) -> Self {
        Self::new_maybe_sync_in_scope_with_caller(value, scope, std::panic::Location::caller())
    }

    /// Create a new CopyValue with a custom caller. The value will be stored in the given scope. When the specified scope is dropped, the value will be dropped.
    #[track_caller]
    pub fn new_maybe_sync_in_scope_with_caller(
        value: T,
        scope: ScopeId,
        caller: &'static std::panic::Location<'static>,
    ) -> Self {
        let owner = Runtime::current().scope_owner(scope);
        Self {
            value: owner.insert_rc_with_caller(value, caller),
            origin_scope: scope,
        }
    }

    /// Manually drop the value in the CopyValue, invalidating the value in the process.
    pub fn manually_drop(&self)
    where
        T: 'static,
    {
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

impl<T, S: Storage<T>> Readable for CopyValue<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        crate::warnings::copy_value_hoisted(self, std::panic::Location::caller());
        self.value.try_read()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        crate::warnings::copy_value_hoisted(self, std::panic::Location::caller());
        self.value.try_read()
    }

    fn subscribers(&self) -> Subscribers {
        Subscribers::new_noop()
    }
}

impl<T, S: Storage<T>> Writable for CopyValue<T, S> {
    type WriteMetadata = ();

    #[track_caller]
    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError> {
        crate::warnings::copy_value_hoisted(self, std::panic::Location::caller());
        self.value.try_write().map(WriteLock::new)
    }
}

impl<T, S: AnyStorage> PartialEq for CopyValue<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.value.ptr_eq(&other.value)
    }
}
impl<T, S: AnyStorage> Eq for CopyValue<T, S> {}

impl<T: Copy + 'static, S: Storage<T>> Deref for CopyValue<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        crate::readable_deref_impl(self)
    }
}

impl<T, S> Clone for CopyValue<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, S> Copy for CopyValue<T, S> {}

read_impls!(CopyValue<T, S: Storage<T>>);
default_impl!(CopyValue<T, S: Storage<T>>);
write_impls!(CopyValue<T, S: Storage<T>>);

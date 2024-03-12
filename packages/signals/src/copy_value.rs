use generational_box::AnyStorage;
use generational_box::GenerationalBoxId;
use generational_box::SyncStorage;
use generational_box::UnsyncStorage;
use std::any::Any;
use std::any::TypeId;
use std::cell::RefCell;
use std::ops::Deref;

use dioxus_core::prelude::*;
use dioxus_core::ScopeId;

use generational_box::{GenerationalBox, Owner, Storage};

use crate::ReadableRef;
use crate::Writable;
use crate::WritableRef;
use crate::{ReactiveContext, Readable};

/// Run a closure with the given owner.
pub fn with_owner<S: AnyStorage, F: FnOnce() -> R, R>(owner: Owner<S>, f: F) -> R {
    let old_owner = set_owner(Some(owner));
    let result = f();
    set_owner(old_owner);
    result
}

/// Set the owner for the current thread.
fn set_owner<S: AnyStorage>(owner: Option<Owner<S>>) -> Option<Owner<S>> {
    let id = TypeId::of::<S>();
    if id == TypeId::of::<SyncStorage>() {
        SYNC_OWNER.with(|cell| {
            std::mem::replace(
                &mut *cell.borrow_mut(),
                owner.map(|owner| {
                    *(Box::new(owner) as Box<dyn Any>)
                        .downcast::<Owner<SyncStorage>>()
                        .unwrap()
                }),
            )
            .map(|owner| *(Box::new(owner) as Box<dyn Any>).downcast().unwrap())
        })
    } else {
        UNSYNC_OWNER.with(|cell| {
            std::mem::replace(
                &mut *cell.borrow_mut(),
                owner.map(|owner| {
                    *(Box::new(owner) as Box<dyn Any>)
                        .downcast::<Owner<UnsyncStorage>>()
                        .unwrap()
                }),
            )
            .map(|owner| *(Box::new(owner) as Box<dyn Any>).downcast().unwrap())
        })
    }
}

thread_local! {
    static SYNC_OWNER: RefCell<Option<Owner<SyncStorage>>> = const { RefCell::new(None) };
    static UNSYNC_OWNER: RefCell<Option<Owner<UnsyncStorage>>> = const { RefCell::new(None) };
}

fn current_owner<S: Storage<T>, T>() -> Owner<S> {
    let id = TypeId::of::<S>();
    let override_owner = if id == TypeId::of::<SyncStorage>() {
        SYNC_OWNER.with(|cell| {
            let owner = cell.borrow();

            owner.clone().map(|owner| {
                *(Box::new(owner) as Box<dyn Any>)
                    .downcast::<Owner<S>>()
                    .unwrap()
            })
        })
    } else {
        UNSYNC_OWNER.with(|cell| {
            cell.borrow().clone().map(|owner| {
                *(Box::new(owner) as Box<dyn Any>)
                    .downcast::<Owner<S>>()
                    .unwrap()
            })
        })
    };
    if let Some(owner) = override_owner {
        return owner;
    }

    // Otherwise get the owner from the current reactive context.
    match ReactiveContext::current() {
        Some(current_reactive_context) => owner_in_scope(current_reactive_context.origin_scope()),
        None => owner_in_scope(current_scope_id().expect("in a virtual dom")),
    }
}

fn owner_in_scope<S: Storage<T>, T>(scope: ScopeId) -> Owner<S> {
    match scope.has_context() {
        Some(rt) => rt,
        None => {
            let owner = S::owner();
            scope.provide_context(owner)
        }
    }
}

/// CopyValue is a wrapper around a value to make the value mutable and Copy.
///
/// It is internally backed by [`generational_box::GenerationalBox`].
pub struct CopyValue<T: 'static, S: Storage<T> = UnsyncStorage> {
    pub(crate) value: GenerationalBox<T, S>,
    origin_scope: ScopeId,
}

#[cfg(feature = "serde")]
impl<T: 'static, Store: Storage<T>> serde::Serialize for CopyValue<T, Store>
where
    T: serde::Serialize,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.read().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
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
        #[cfg(debug_assertions)] caller: &'static std::panic::Location<'static>,
    ) -> Self {
        let owner = current_owner();

        Self {
            value: owner.insert_with_caller(
                value,
                #[cfg(debug_assertions)]
                caller,
            ),
            origin_scope: current_scope_id().expect("in a virtual dom"),
        }
    }

    /// Create a new CopyValue. The value will be stored in the given scope. When the specified scope is dropped, the value will be dropped.
    #[track_caller]
    pub fn new_maybe_sync_in_scope(value: T, scope: ScopeId) -> Self {
        let owner = owner_in_scope(scope);

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
}

impl<T: 'static, S: Storage<T>> Readable for CopyValue<T, S> {
    type Target = T;
    type Storage = S;

    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        self.value.try_read()
    }

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

//! Integration with the generational-box crate for copy state management.
//!
//! Each scope in dioxus has a single [Owner]

use generational_box::{AnyStorage, Owner, SyncStorage, UnsyncStorage};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
};

/// Run a closure with the given owner.
///
/// This will override the default owner for the current component.
pub fn with_owner<S: AnyStorage, F: FnOnce() -> R, R>(owner: Owner<S>, f: F) -> R {
    let old_owner = set_owner(Some(owner));
    let result = f();
    set_owner(old_owner);
    result
}

/// Owners for both storage types. Generated props structs hold one of these so that any
/// state allocated while converting props is dropped with the props, regardless of
/// which storage backs the state.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct PropsOwner {
    unsync: Owner<UnsyncStorage>,
    sync: Owner<SyncStorage>,
}

/// Run a closure with both owners of the given [`PropsOwner`].
///
/// This will override the default owner for the current component for both storage types.
#[doc(hidden)]
pub fn with_props_owner<F: FnOnce() -> R, R>(owner: PropsOwner, f: F) -> R {
    let PropsOwner { unsync, sync } = owner;
    with_owner(unsync, move || with_owner(sync, f))
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

/// Returns the current owner. This owner will be used to drop any `Copy` state that is created by the `generational-box` crate.
///
/// If an owner has been set with `with_owner`, that owner will be returned. Otherwise, the owner from the current scope will be returned.
pub fn current_owner<S: AnyStorage>() -> Owner<S> {
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

    crate::Runtime::current().current_owner()
}

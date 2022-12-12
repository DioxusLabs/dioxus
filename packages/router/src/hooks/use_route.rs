use async_rwlock::RwLockReadGuard;
use dioxus::{core::Component, prelude::ScopeState};
use dioxus_router_core::RouterState;
use log::error;

use crate::utils::use_router_internal::use_router_internal;

#[must_use]
pub fn use_route<'a>(cx: &'a ScopeState) -> Option<RwLockReadGuard<'a, RouterState<Component>>> {
    match use_router_internal(cx) {
        Some(r) => loop {
            if let Some(s) = r.state.try_read() {
                break Some(s);
            }
        },
        None => {
            let msg = "`use_route` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            #[cfg(not(debug_assertions))]
            None
        }
    }
}

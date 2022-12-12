use dioxus::prelude::{ScopeId, ScopeState};
use dioxus_router_core::Navigator;
use log::error;

use crate::utils::use_router_internal::use_router_internal;

#[must_use]
pub fn use_navigate(cx: &ScopeState) -> Option<Navigator<ScopeId>> {
    match use_router_internal(cx) {
        Some(r) => Some(r.sender.clone().into()),
        None => {
            let msg = "`use_navigate` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            #[cfg(not(debug_assertions))]
            None
        }
    }
}

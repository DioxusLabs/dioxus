use dioxus::prelude::ScopeState;

use crate::{prelude::RouterContext, utils::use_router_internal::use_router_internal};

#[deprecated = "prefer the `router()` function or `use_route` functions"]
#[must_use]
/// A hook that provides access to information about the router.
pub fn use_router(cx: &ScopeState) -> &RouterContext {
    use_router_internal(cx)
        .as_ref()
        .expect("use_route must have access to a router")
}

/// Aquire the router without subscribing to updates.
pub fn router() -> RouterContext {
    dioxus::core::prelude::consume_context().unwrap()
}

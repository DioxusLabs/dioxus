use crate::{utils::use_router_internal::use_router_internal, RouterContext};

#[deprecated = "prefer the `router()` function or `use_route` functions"]
#[must_use]
/// A hook that provides access to information about the router.
pub fn use_router() -> RouterContext {
    use_router_internal().expect("use_route must have access to a router")
}

/// Acquire the router without subscribing to updates.
#[doc(alias = "url")]
pub fn router() -> RouterContext {
    dioxus_core::consume_context()
}

/// Try to acquire the router without subscribing to updates.
#[doc(alias = "url")]
pub fn try_router() -> Option<RouterContext> {
    dioxus_core::try_consume_context()
}

use dioxus::prelude::ScopeState;

use crate::{
    prelude::GenericRouterContext, routable::Routable,
    utils::use_router_internal::use_router_internal,
};

/// A hook that provides access to information about the router
pub fn use_generic_router<R: Routable + Clone>(cx: &ScopeState) -> &GenericRouterContext<R> {
    use_router_internal(cx)
        .as_ref()
        .expect("use_route must have access to a router")
}

use crate::RouterContext;
use dioxus::{core::ScopeState, prelude::use_context};

/// This hook provides access to the `RouterService` for the app.
pub fn use_router(cx: &ScopeState) -> &RouterContext {
    use_context::<RouterContext>(cx)
        .expect("Cannot call use_route outside the scope of a Router component")
}

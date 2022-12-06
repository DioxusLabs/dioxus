use std::sync::Arc;

use dioxus::prelude::ScopeState;
use dioxus_router_core::RouterMessage;

use crate::contexts::router::RouterContext;

/// A private hook to subscribe to the router.
///
/// Used to reduce redundancy within other components/hooks. Safe to call multiple times for a
/// single component, but not recommended. Multiple subscriptions will be discarded.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a [`use_router`] component.
/// - Otherwise [`Some`].
///
/// [`use_router`]: crate::hooks::use_router
pub(crate) fn use_router_internal<'a>(cx: &'a ScopeState) -> &'a mut Option<RouterContext> {
    let id = cx.use_hook(|| Arc::new(cx.scope_id()));

    cx.use_hook(|| {
        let router = cx.consume_context::<RouterContext>()?;

        let _ = router
            .sender
            .unbounded_send(RouterMessage::Subscribe(id.clone()));

        Some(router)
    })
}

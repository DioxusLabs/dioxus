//! Helpers that provide common functionality.

use std::sync::Arc;

use dioxus_core::ScopeState;

use crate::contexts::RouterContext;

/// A private "hook" to subscribe to the router.
///
/// Used to reduce redundancy within other components/hooks. Safe to call multiple times for a
/// single component, but not recommended.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a
///   [Router](crate::components::Router).
/// - Otherwise [`Some`].
pub(crate) fn sub_to_router<'a>(cx: &'a ScopeState) -> &'a mut Option<RouterContext> {
    let id = cx.use_hook(|_| Arc::new(cx.scope_id()));

    cx.use_hook(|_| {
        let router = cx.consume_context::<RouterContext>()?;

        router
            .tx
            .unbounded_send(crate::service::RouterMessage::Subscribe(id.clone()))
            .unwrap();

        Some(router)
    })
}

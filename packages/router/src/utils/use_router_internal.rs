use crate::RouterContext;
use dioxus_core::{try_consume_context, use_hook};

/// A private hook to subscribe to the router.
///
/// Used to reduce redundancy within other components/hooks. Safe to call multiple times for a
/// single component, but not recommended. Multiple subscriptions will be discarded.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a [`crate::Router`] component.
/// - Otherwise [`Some`].
pub(crate) fn use_router_internal() -> Option<RouterContext> {
    use_hook(try_consume_context)
}

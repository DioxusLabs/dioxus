use dioxus_lib::prelude::*;

use crate::prelude::*;

/// A private hook to subscribe to the router.
///
/// Used to reduce redundancy within other components/hooks. Safe to call multiple times for a
/// single component, but not recommended. Multiple subscriptions will be discarded.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a [`Router`] component.
/// - Otherwise [`Some`].
pub(crate) fn use_router_internal() -> Option<RouterContext> {
    use_hook(try_consume_context)
}

use dioxus::prelude::{ScopeId, ScopeState};

use crate::prelude::*;

/// A private hook to subscribe to the router.
///
/// Used to reduce redundancy within other components/hooks. Safe to call multiple times for a
/// single component, but not recommended. Multiple subscriptions will be discarded.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a [`Link`] component.
/// - Otherwise [`Some`].
pub(crate) fn use_router_internal(cx: &ScopeState) -> &Option<RouterContext> {
    let inner = cx.use_hook(|| {
        let router = cx.consume_context::<RouterContext>()?;

        let id = cx.scope_id();
        router.subscribe(id);

        Some(Subscription { router, id })
    });
    cx.use_hook(|| inner.as_ref().map(|s| s.router.clone()))
}

struct Subscription {
    router: RouterContext,
    id: ScopeId,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.router.unsubscribe(self.id);
    }
}

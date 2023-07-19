use dioxus::prelude::{ScopeId, ScopeState};

use crate::{contexts::router::GenericRouterContext, prelude::*};

/// A private hook to subscribe to the router.
///
/// Used to reduce redundancy within other components/hooks. Safe to call multiple times for a
/// single component, but not recommended. Multiple subscriptions will be discarded.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a [`GenericRouter`] component.
/// - Otherwise [`Some`].
pub(crate) fn use_router_internal<R: Routable>(
    cx: &ScopeState,
) -> &Option<GenericRouterContext<R>> {
    let inner = cx.use_hook(|| {
        let router = cx.consume_context::<GenericRouterContext<R>>()?;

        let id = cx.scope_id();
        router.subscribe(id);

        Some(Subscription { router, id })
    });
    cx.use_hook(|| inner.as_ref().map(|s| s.router.clone()))
}

struct Subscription<R: Routable> {
    router: GenericRouterContext<R>,
    id: ScopeId,
}

impl<R: Routable> Drop for Subscription<R> {
    fn drop(&mut self) {
        self.router.unsubscribe(self.id);
    }
}

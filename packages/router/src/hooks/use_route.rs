use dioxus_core::{ScopeId, ScopeState};
use std::{rc::Rc, str::FromStr, sync::Arc};

use crate::{location::ParsedRoute, RouterCore, RouterService};

/// This hook provides access to information about the current location in the
/// context of a [`Router`]. If this function is called outside of a `Router`
/// component it will panic.
pub fn use_route(cx: &ScopeState) -> &ParsedRoute {
    let handle = cx.use_hook(|_| {
        let router = cx
            .consume_context::<Arc<RouterCore>>()
            .expect("Cannot call use_route outside the scope of a Router component");

        router.subscribe_onchange(cx.scope_id());

        UseRouteListener {
            route: router.current_location(),
            router,
            scope: cx.scope_id(),
        }
    });

    &handle.route
}

// The entire purpose of this struct is to unubscribe this component when it is unmounted.
// The UseRoute can be cloned into async contexts, so we can't rely on its drop to unubscribe.
// Instead, we hide the drop implementation on this private type exclusive to the hook,
// and reveal our cached version of UseRoute to the component.
struct UseRouteListener {
    route: Arc<ParsedRoute>,
    router: Arc<RouterCore>,
    scope: ScopeId,
}

impl Drop for UseRouteListener {
    fn drop(&mut self) {
        self.router.unsubscribe_onchange(self.scope)
    }
}

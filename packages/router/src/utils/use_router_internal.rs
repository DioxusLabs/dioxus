use dioxus_lib::prelude::*;

use crate::prelude::*;

/// A private hook to subscribe to the router.
///
/// Used to reduce redundancy within other components/hooks. Safe to call multiple times for a
/// single component, but not recommended. Multiple subscriptions will be discarded.
///
/// # Return values
/// - [`None`], when the current component isn't a descendant of a [`Link`] component.
/// - Otherwise [`Some`].
pub(crate) fn use_router_internal() -> Option<RouterContext> {
    let router = use_hook(consume_context::<RouterContext>);
    use_on_destroy({
        to_owned![router];
        move || {
            let id = current_scope_id().expect("use_router_internal called outside of a component");

            router.unsubscribe(id);
        }
    });
    use_hook(move || {
        let id = current_scope_id().expect("use_router_internal called outside of a component");
        router.subscribe(id);

        Some(router)
    })
}

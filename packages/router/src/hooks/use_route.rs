use std::sync::RwLockReadGuard;

use dioxus_core::ScopeState;
use log::error;

use crate::{helpers::sub_to_router, state::RouterState};

/// A hook that allows you access to information about the currently active route.
///
/// # Return values
/// - [`None`], when the calling component is not nested within a [`Router`].
/// - Otherwise [`Some`].
///
/// # Important usage information
/// Make sure to [`drop`] the returned [`RwLockReadGuard`] when your component is done rendering.
/// Otherwise you prevent the router from updating the data when navigating.
///
/// # Panic
/// - When the calling component is not nested within a [`Router`], but only in debug builds.
///
/// # Example
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// fn SomeComponent(cx: Scope) -> Element {
///     let route = use_route(&cx).expect("router as ancestor");
///     let path = &route.path;
///
///     cx.render(rsx! {
///         p { "current path: {path}" }
///     })
/// }
/// ```
///
/// [`Router`]: crate::components::Router
#[must_use]
pub fn use_route(cx: &ScopeState) -> Option<RwLockReadGuard<RouterState>> {
    let router = sub_to_router(cx);

    match router {
        Some(r) => Some(r.state.read().unwrap()),
        None => {
            error!("`use_route` can only be used in descendants of a `Router`");
            #[cfg(debug_assertions)]
            panic!("`use_route` can only be used in descendants of a `Router`");
            #[cfg(not(debug_assertions))]
            None
        }
    }
}

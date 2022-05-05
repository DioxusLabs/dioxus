use std::sync::RwLockReadGuard;

use dioxus_core::ScopeState;
use log::error;

use crate::{helpers::sub_to_router, state::RouterState};

/// A hook that allows you access to information about the currently active route.
///
/// # Return values
/// - [`None`], when the current component isn't a descendent of a [`Router`].
/// - Otherwise [`Some`].
///
/// [`Router`]: crate::components::Router
///
/// # Important usage information
/// Make sure to [`drop`] the returned [`RwLockReadGuard`] when your component is done rendering.
/// Otherwise you prevent the router from updating the data when navigating.
#[must_use]
pub fn use_route(cx: &ScopeState) -> Option<RwLockReadGuard<RouterState>> {
    let router = sub_to_router(cx);

    if router.is_none() {
        error!("`use_route` can only be used in descendants of a `Router`");
    }

    match router {
        Some(r) => Some(r.state.read().unwrap()),
        None => None,
    }
}

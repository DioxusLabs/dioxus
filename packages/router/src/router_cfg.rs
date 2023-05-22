use crate::contexts::router::RoutingCallback;
use crate::history::{HistoryProvider, MemoryHistory};
use crate::routable::Routable;
use dioxus::prelude::*;

use crate::prelude::default_errors::{
    FailureExternalNavigation, FailureNamedNavigation, FailureRedirectionLimit,
};

/// Global configuration options for the router.
///
/// This implements [`Default`], so you can use it like this:
/// ```rust,no_run
/// # use dioxus_router::prelude::RouterConfiguration;
/// let cfg = RouterConfiguration {
///     synchronous: false,
///     ..Default::default()
/// };
/// ```
pub struct RouterConfiguration<R: Routable> {
    /// A component to render when an external navigation fails.
    ///
    /// Defaults to a router-internal component called `FailureExternalNavigation`. It is not part
    /// of the public API. Do not confuse it with
    /// [`dioxus_router_core::prelude::FailureExternalNavigation`].
    pub failure_external_navigation: fn(Scope) -> Element,
    /// A component to render when a named navigation fails.
    ///
    /// Defaults to a router-internal component called `FailureNamedNavigation`. It is not part of
    /// the public API. Do not confuse it with
    /// [`dioxus_router_core::prelude::FailureNamedNavigation`].
    pub failure_named_navigation: fn(Scope) -> Element,
    /// A component to render when the redirect limit is reached.
    ///
    /// Defaults to a router-internal component called `FailureRedirectionLimit`. It is not part of
    /// the public API. Do not confuse it with
    /// [`dioxus_router_core::prelude::FailureRedirectionLimit`].
    pub failure_redirection_limit: fn(Scope) -> Element,
    /// The [`HistoryProvider`] the router should use.
    ///
    /// Defaults to a default [`MemoryHistory`].
    pub history: Box<dyn HistoryProvider<R>>,
    /// A function to be called whenever the routing is updated.
    ///
    /// The callback is invoked after the routing is updated, but before components and hooks are
    /// updated.
    ///
    /// If the callback returns a [`NavigationTarget`] the router will replace the current location
    /// with it. If no navigation failure was triggered, the router will then updated dependent
    /// components and hooks.
    ///
    /// The callback is called no more than once per rerouting. It will not be called if a
    /// navigation failure occurs.
    ///
    /// Defaults to [`None`].
    ///
    /// [`NavigationTarget`]: dioxus_router_core::navigation::NavigationTarget
    pub on_update: Option<RoutingCallback<R>>,
}

impl<R: Routable + Clone> Default for RouterConfiguration<R> {
    fn default() -> Self {
        Self {
            failure_external_navigation: FailureExternalNavigation::<R>,
            failure_named_navigation: FailureNamedNavigation::<R>,
            failure_redirection_limit: FailureRedirectionLimit::<R>,
            history: Box::<MemoryHistory<R>>::default(),
            on_update: None,
        }
    }
}

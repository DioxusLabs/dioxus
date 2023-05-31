use crate::contexts::router::RoutingCallback;
use crate::history::HistoryProvider;
use crate::routable::Routable;
use dioxus::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

use crate::prelude::*;

/// Global configuration options for the router.
///
/// This implements [`Default`], so you can use it like this:
/// ```rust,no_run
/// # use dioxus_router::prelude::*;
/// # use serde::{Deserialize, Serialize};
/// # use dioxus::prelude::*;
/// # #[inline_props]
/// # fn Index(cx: Scope) -> Element {
/// #     todo!()
/// # }
/// #[derive(Clone, Serialize, Deserialize, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
/// let cfg = RouterConfiguration {
///     history: Box::<WebHistory<Route>>::default(),
///     ..Default::default()
/// };
/// ```
pub struct RouterConfiguration<R: Routable> {
    /// A component to render when an external navigation fails.
    ///
    /// Defaults to a router-internal component called [`FailureExternalNavigation`]
    pub failure_external_navigation: fn(Scope) -> Element,
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
    pub on_update: Option<RoutingCallback<R>>,
}

impl<R: Routable + Clone + Serialize + DeserializeOwned> Default for RouterConfiguration<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            failure_external_navigation: FailureExternalNavigation::<R>,
            history: Box::<MemoryHistory<R>>::default(),
            on_update: None,
        }
    }
}

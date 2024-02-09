use std::sync::Arc;

use crate::contexts::router::RoutingCallback;
use crate::history::HistoryProvider;
use crate::routable::Routable;
use dioxus_lib::prelude::*;

use crate::prelude::*;

/// Global configuration options for the router.
///
/// This implements [`Default`] and follows the builder pattern, so you can use it like this:
/// ```rust,no_run
/// # use dioxus_router::prelude::*;
/// # use dioxus::prelude::*;
/// # #[component]
/// # fn Index() -> Element {
/// #     None
/// # }
/// #[derive(Clone, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
/// let cfg = RouterConfig::default().history(WebHistory::<Route>::default());
/// ```
pub struct RouterConfig<R: Routable> {
    pub(crate) failure_external_navigation: fn() -> Element,
    pub(crate) history: Option<Box<dyn AnyHistoryProvider>>,
    pub(crate) on_update: Option<RoutingCallback<R>>,
    pub(crate) initial_route: Option<R>,
}

macro_rules! default_history {
    ($initial_route:ident) => {
        {
            // If we are on wasm32 and the web feature is enabled, use the web history.
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            return Box::new(AnyHistoryProviderImplWrapper::new(WebHistory::<R>::default()));
            // If we are using dioxus fullstack and the ssr feature is enabled, use the memory history with the initial path set to the current path in fullstack
            #[cfg(all(feature = "fullstack", feature = "ssr"))]
            return Box::new(AnyHistoryProviderImplWrapper::new(MemoryHistory::<R>::with_initial_path(
                dioxus_fullstack::prelude::server_context()
                    .request_parts()
                    .unwrap()
                    .uri
                    .to_string()
                    .parse()
                    .unwrap_or_else(|err| {
                        tracing::error!("Failed to parse uri: {}", err);
                        "/"
                            .parse()
                            .unwrap_or_else(|err| {
                                panic!("Failed to parse uri: {}", err);
                            })
                    }),
            )));
            // If we are not on wasm32 and the liveview feature is enabled, use the liveview history.
            #[cfg(all(feature = "liveview"))]
            return Box::new(AnyHistoryProviderImplWrapper::new(LiveviewHistory::new_with_initial_path($initial_route)));
            // Otherwise use the memory history.
            #[cfg(all(
                not(all(target_arch = "wasm32", feature = "web")),
                not(all(feature = "liveview", not(target_arch = "wasm32"))),
            ))]
            Box::new(AnyHistoryProviderImplWrapper::new(MemoryHistory::with_initial_path($initial_route)))
        }
    };
}

impl<R: Routable + Clone> Default for RouterConfig<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            failure_external_navigation: FailureExternalNavigation,
            history: None,
            on_update: None,
            initial_route: None,
        }
    }
}

#[cfg(not(feature = "serde"))]
impl<R: Routable + Clone> RouterConfig<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    pub(crate) fn take_history(&mut self) -> Box<dyn AnyHistoryProvider> {
        #[allow(unused)]
        let initial_route = self.initial_route.clone().unwrap_or("/".parse().unwrap_or_else(|err|
            panic!("index route does not exist:\n{}\n use MemoryHistory::with_initial_path or RouterConfig::initial_route to set a custom path", err)
        ));
        self.history
            .take()
            .unwrap_or_else(|| default_history!(initial_route))
    }
}

impl<R> RouterConfig<R>
where
    R: Routable,
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
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
    pub fn on_update(
        self,
        callback: impl Fn(GenericRouterContext<R>) -> Option<NavigationTarget<R>> + 'static,
    ) -> Self {
        Self {
            on_update: Some(Arc::new(callback)),
            ..self
        }
    }

    /// The [`HistoryProvider`] the router should use.
    ///
    /// Defaults to a different history provider depending on the target platform.
    pub fn history(self, history: impl HistoryProvider<R> + 'static) -> Self {
        Self {
            history: Some(Box::new(AnyHistoryProviderImplWrapper::new(history))),
            ..self
        }
    }

    /// The initial route the router should use if no history provider is set.
    pub fn initial_route(self, route: R) -> Self {
        Self {
            initial_route: Some(route),
            ..self
        }
    }

    /// A component to render when an external navigation fails.
    ///
    /// Defaults to a router-internal component called [`FailureExternalNavigation`]
    pub fn failure_external_navigation(self, component: fn() -> Element) -> Self {
        Self {
            failure_external_navigation: component,
            ..self
        }
    }
}

use crate::prelude::*;
use dioxus_document::DocumentContext;
use dioxus_lib::prelude::*;
use generic_router::GenericRouterContext;
use std::sync::Arc;

/// Global configuration options for the router.
///
/// This implements [`Default`] and follows the builder pattern, so you can use it like this:
/// ```rust,no_run
/// # use dioxus_router::prelude::*;
/// # use dioxus::prelude::*;
/// # #[component]
/// # fn Index() -> Element {
/// #     VNode::empty()
/// # }
/// #[derive(Clone, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
/// let cfg = RouterConfig::default().history(MemoryHistory::<Route>::default());
/// ```
pub struct RouterConfig<R: Routable> {
    pub(crate) failure_external_navigation: fn() -> Element,
    pub(crate) history: Option<DocumentContext>,
    pub(crate) on_update: Option<RoutingCallback<R>>,
    pub(crate) initial_route: Option<R>,
}

impl<R: Routable> Default for RouterConfig<R> {
    fn default() -> Self {
        Self {
            failure_external_navigation: FailureExternalNavigation,
            history: None,
            on_update: None,
            initial_route: None,
        }
    }
}

// impl<R: Routable> RouterConfig<R>
// where
//     <R as std::str::FromStr>::Err: std::fmt::Display,
// {
//     pub(crate) fn take_history(&mut self) -> Box<dyn AnyHistoryProvider> {
//         self.history
//             .take()
//             .unwrap_or_else(|| {
//                 let initial_route = self.initial_route.clone().unwrap_or_else(|| "/".parse().unwrap_or_else(|err|
//                     panic!("index route does not exist:\n{}\n use MemoryHistory::with_initial_path or RouterConfig::initial_route to set a custom path", err)
//                 ));
//                 default_history(initial_route)
//     })
//     }
// }

impl<R> RouterConfig<R>
where
    R: Routable,
    // <R as std::str::FromStr>::Err: std::fmt::Display,
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

    pub fn with_initial_path(self, initial_path: R) -> Self {
        Self {
            initial_route: Some(initial_path),
            ..self
        }
    }

    // /// The [`HistoryProvider`] the router should use.
    // ///
    // /// Defaults to a different history provider depending on the target platform.
    // pub fn history(self, history: impl HistoryProvider<R> + 'static) -> Self {
    //     Self {
    //         history: Some(Box::new(AnyHistoryProviderImplWrapper::new(history))),
    //         ..self
    //     }
    // }

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

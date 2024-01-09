use std::sync::Arc;

use crate::contexts::router::RoutingCallback;
use crate::history::HistoryProvider;
use crate::routable::Routable;
use dioxus::prelude::*;

use crate::prelude::*;

/// Global configuration options for the router.
///
/// This implements [`Default`] and follows the builder pattern, so you can use it like this:
/// ```rust,no_run
/// # use dioxus_router::prelude::*;
/// # use dioxus::prelude::*;
/// # #[component]
/// # fn Index(cx: Scope) -> Element {
/// #     todo!()
/// # }
/// #[derive(Clone, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
/// let cfg = RouterConfig::default().history(WebHistory::<Route>::default());
/// ```
pub struct RouterConfig<R: Routable> {
    pub(crate) failure_external_navigation: fn(Scope) -> Element,
    pub(crate) history: Option<Box<dyn AnyHistoryProvider>>,
    pub(crate) on_update: Option<RoutingCallback<R>>,
}

#[cfg(feature = "serde")]
impl<R: Routable + Clone> Default for RouterConfig<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
    R: serde::Serialize + serde::de::DeserializeOwned,
{
    fn default() -> Self {
        Self {
            failure_external_navigation: FailureExternalNavigation::<R>,
            history: None,
            on_update: None,
        }
    }
}

#[cfg(feature = "serde")]
impl<R: Routable + Clone> RouterConfig<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
    R: serde::Serialize + serde::de::DeserializeOwned,
{
    pub(crate) fn get_history(self) -> Box<dyn HistoryProvider<R>> {
        self.history.unwrap_or_else(|| {
            #[cfg(all(not(feature = "liveview"), target_arch = "wasm32", feature = "web"))]
            let history = Box::<WebHistory<R>>::default();
            #[cfg(all(
                not(feature = "liveview"),
                any(not(target_arch = "wasm32"), not(feature = "web"))
            ))]
            let history = Box::<MemoryHistory<R>>::default();
            #[cfg(feature = "liveview")]
            let history = Box::<LiveviewHistory<R>>::default();
            history
        })
    }
}

#[cfg(not(feature = "serde"))]
impl<R: Routable + Clone> Default for RouterConfig<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            failure_external_navigation: FailureExternalNavigation,
            history: None,
            on_update: None,
        }
    }
}

#[cfg(not(feature = "serde"))]
impl<R: Routable + Clone> RouterConfig<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    pub(crate) fn take_history(&mut self) -> Box<dyn AnyHistoryProvider> {
        self.history.take().unwrap_or_else(|| {
            // If we are on wasm32 and the web feature is enabled, use the web history.
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            let history = Box::<AnyHistoryProviderImplWrapper<R, WebHistory<R>>>::default();
            // If we are not on wasm32 and the liveview feature is enabled, use the liveview history.
            #[cfg(all(feature = "liveview", not(target_arch = "wasm32")))]
            let history = Box::<AnyHistoryProviderImplWrapper<R, LiveviewHistory<R>>>::default();
            // If neither of the above are true, use the memory history.
            #[cfg(all(
                not(all(target_arch = "wasm32", feature = "web")),
                not(all(feature = "liveview", not(target_arch = "wasm32"))),
            ))]
            let history = Box::<AnyHistoryProviderImplWrapper<R, MemoryHistory<R>>>::default();
            history
        })
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
    /// Defaults to a default [`MemoryHistory`].
    pub fn history(self, history: impl HistoryProvider<R> + 'static) -> Self {
        Self {
            history: Some(Box::new(AnyHistoryProviderImplWrapper::new(history))),
            ..self
        }
    }

    /// A component to render when an external navigation fails.
    ///
    /// Defaults to a router-internal component called [`FailureExternalNavigation`]
    pub fn failure_external_navigation(self, component: fn(Scope) -> Element) -> Self {
        Self {
            failure_external_navigation: component,
            ..self
        }
    }
}

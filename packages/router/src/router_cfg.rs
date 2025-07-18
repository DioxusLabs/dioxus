use crate::{GenericRouterContext, NavigationTarget, Routable, RoutingCallback};
use dioxus_core::Element;
use std::sync::Arc;

/// Global configuration options for the router.
///
/// This implements [`Default`] and follows the builder pattern, so you can use it like this:
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// # use dioxus_router::RouterConfig;
/// # #[component]
/// # fn Index() -> Element {
/// #     VNode::empty()
/// # }
/// #[derive(Clone, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
///
/// fn ExternalNavigationFailure() -> Element {
///     rsx! {
///         "Failed to navigate to external URL"
///     }
/// }
///
/// let cfg = RouterConfig::<Route>::default().failure_external_navigation(ExternalNavigationFailure);
/// ```
pub struct RouterConfig<R> {
    pub(crate) failure_external_navigation: fn() -> Element,
    pub(crate) on_update: Option<RoutingCallback<R>>,
}

#[cfg(not(feature = "html"))]
impl<R> Default for RouterConfig<R> {
    fn default() -> Self {
        Self {
            failure_external_navigation: || VNode::empty(),
            on_update: None,
        }
    }
}

#[cfg(feature = "html")]
impl<R> Default for RouterConfig<R> {
    fn default() -> Self {
        Self {
            failure_external_navigation: crate::components::FailureExternalNavigation,
            on_update: None,
        }
    }
}

impl<R> RouterConfig<R>
where
    R: Routable,
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

    /// A component to render when an external navigation fails.
    ///
    #[cfg_attr(
        feature = "html",
        doc = "Defaults to [`crate::components::FailureExternalNavigation`]."
    )]
    pub fn failure_external_navigation(self, component: fn() -> Element) -> Self {
        Self {
            failure_external_navigation: component,
            ..self
        }
    }
}

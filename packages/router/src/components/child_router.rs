/// Components that allow the macro to add child routers. This component provides a context
/// to the child router that maps child routes to root routes and vice versa.
use crate::{Outlet, OutletContext, Routable};
use dioxus_core::{Element, provide_context, try_consume_context, use_hook};
use dioxus_core_macro::{Props, component, rsx};
use std::sync::Arc;

/// Maps a child route into the root router and vice versa
// `Arc<dyn Fn>` rather than a fn pointer so the derive macro can capture parent dynamic-segment values
pub(crate) struct ChildRouteMapping<R> {
    format_route_as_root_route: Arc<dyn Fn(R) -> String>,
    parse_route_from_root_route: fn(&str) -> Option<R>,
}

impl<R: Routable> ChildRouteMapping<R> {
    pub(crate) fn format_route_as_root_route(&self, route: R) -> String {
        (self.format_route_as_root_route)(route)
    }

    pub(crate) fn parse_route_from_root_route(&self, route: &str) -> Option<R> {
        (self.parse_route_from_root_route)(route)
    }
}

/// Get the formatter that handles adding and stripping the prefix from a child route
pub(crate) fn consume_child_route_mapping<R: Routable>() -> Option<ChildRouteMapping<R>> {
    try_consume_context()
}

/// Parse an absolute URL into `R`, walking any outer `ChildRouteMapping<R>` chain.
pub fn parse_route_via_chain<R: Routable>(route: &str) -> Option<R> {
    consume_child_route_mapping::<R>()
        .and_then(|outer| outer.parse_route_from_root_route(route))
        .or_else(|| route.parse().ok())
}

/// Format `value` as an absolute URL, walking any outer `ChildRouteMapping<R>` chain.
pub fn format_route_via_chain<R: Routable>(value: R) -> String {
    match consume_child_route_mapping::<R>() {
        Some(outer) => outer.format_route_as_root_route(value),
        None => value.to_string(),
    }
}

impl<R> Clone for ChildRouteMapping<R> {
    fn clone(&self) -> Self {
        Self {
            format_route_as_root_route: Arc::clone(&self.format_route_as_root_route),
            parse_route_from_root_route: self.parse_route_from_root_route,
        }
    }
}

/// Props for the [`ChildRouter`] component.
#[derive(Props, Clone)]
pub struct ChildRouterProps<R: Routable> {
    /// The child route to render
    route: R,
    /// Take a parent route and return a child route or none if the route is not part of the child
    parse_route_from_root_route: fn(&str) -> Option<R>,
    /// Take a child route and return a parent route
    format_route_as_root_route: Arc<dyn Fn(R) -> String>,
}

impl<R: Routable> PartialEq for ChildRouterProps<R> {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

/// A component that provides a [`History`](dioxus_history::History) to a child router. The `#[child]` attribute on the router macro will insert this automatically.
#[component]
#[allow(missing_docs)]
pub fn ChildRouter<R: Routable>(props: ChildRouterProps<R>) -> Element {
    use_hook(|| {
        provide_context(ChildRouteMapping {
            format_route_as_root_route: props.format_route_as_root_route,
            parse_route_from_root_route: props.parse_route_from_root_route,
        });
        provide_context(OutletContext::<R>::new());
    });

    rsx! { Outlet::<R> {} }
}

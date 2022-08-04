use std::sync::Arc;

use dioxus::prelude::*;

use crate::route_definition::Segment;

/// A hook that makes constructing the [`Segment`] for a [`Router`] easier.
///
/// # Example
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// fn App(cx: Scope) -> Element {
///     // create the routes
///     let routes = use_segment(&cx, || {
///         Segment::new()
///     });
///
///     cx.render(rsx! {
///         Router {
///             // pass the routes to the router
///             routes: routes.clone(),
///             Outlet { }
///         }
///     })
/// }
/// ```
///
/// [`Router`]: crate::components::Router
pub fn use_segment(cx: &ScopeState, init: impl FnOnce() -> Segment) -> &'_ Arc<Segment> {
    cx.use_hook(|| Arc::new(init()))
}

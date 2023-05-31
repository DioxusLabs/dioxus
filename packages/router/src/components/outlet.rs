use crate::prelude::{outlet::OutletContext, *};
use dioxus::prelude::*;

/// An outlet for the current content.
///
/// Only works as descendant of a [`GenericRouter`] component, otherwise it will be inactive.
///
/// The [`GenericOutlet`] is aware of how many [`Outlet`]s it is nested within. It will render the content
/// of the active route that is __exactly as deep__.
///
/// # Panic
/// - When the [`GenericOutlet`] is not nested a [`GenericRouter`] component,
///   but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use serde::{Deserialize, Serialize};
/// # use dioxus_router::prelude::*;
/// #[derive(Clone, Serialize, Deserialize, Routable)]
/// #[rustfmt::skip]
/// enum Route {
///     #[nest("/wrap")]
///         #[layout(Wrapper)] // Every layout component must have one Outlet
///             #[route("/")]
///             Child {},
///         #[end_layout]
///     #[end_nest]
///     #[route("/")]
///     Index {},
/// }
///
/// #[inline_props]
/// fn Index(cx: Scope) -> Element {
///     render! {
///         div {
///             "Index"
///         }
///     }
/// }
///
/// #[inline_props]
/// fn Wrapper(cx: Scope) -> Element {
///     render! {
///         h1 { "App" }
///         Outlet {} // The content of child routes will be rendered here
///     }
/// }
///
/// #[inline_props]
/// fn Child(cx: Scope) -> Element {
///     render! {
///         p {
///             "Child"
///         }
///     }
/// }
///
/// # fn App(cx: Scope) -> Element {
/// #     render! {
/// #         Router {
/// #             config: RouterConfiguration {
/// #                 history: Box::new(MemoryHistory::with_initial_path("/wrap").unwrap()),
/// #                 ..Default::default()
/// #             }
/// #         }
/// #     }
/// # }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><p>Child</p>");
/// ```
pub fn GenericOutlet<R: Routable + Clone>(cx: Scope) -> Element {
    OutletContext::render::<R>(cx)
}

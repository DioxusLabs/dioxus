use crate::prelude::{outlet::OutletContext, *};
use dioxus::prelude::*;

/// An outlet for the current content.
///
/// Only works as descendant of a [`Link`] component, otherwise it will be inactive.
///
/// The [`Outlet`] is aware of how many [`Outlet`]s it is nested within. It will render the content
/// of the active route that is __exactly as deep__.
///
/// # Panic
/// - When the [`Outlet`] is not nested a [`Link`] component,
///   but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// #[derive(Clone, Routable)]
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
/// #[component]
/// fn Index(cx: Scope) -> Element {
///     render! {
///         div {
///             "Index"
///         }
///     }
/// }
///
/// #[component]
/// fn Wrapper(cx: Scope) -> Element {
///     render! {
///         h1 { "App" }
///         Outlet::<Route> {} // The content of child routes will be rendered here
///     }
/// }
///
/// #[component]
/// fn Child(cx: Scope) -> Element {
///     render! {
///         p {
///             "Child"
///         }
///     }
/// }
///
/// # #[component]
/// # fn App(cx: Scope) -> Element {
/// #     render! {
/// #         Router::<Route> {
/// #             config: || RouterConfig::default().history(MemoryHistory::with_initial_path(Route::Child {}))
/// #         }
/// #     }
/// # }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><p>Child</p>");
/// ```
pub fn Outlet<R: Routable + Clone>(cx: Scope) -> Element {
    OutletContext::<R>::render(cx)
}

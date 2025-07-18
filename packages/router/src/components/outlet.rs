use crate::{outlet::OutletContext, *};
use dioxus_core::Element;

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
/// fn Index() -> Element {
///     rsx! {
///         div {
///             "Index"
///         }
///     }
/// }
///
/// #[component]
/// fn Wrapper() -> Element {
///     rsx! {
///         h1 { "App" }
///         Outlet::<Route> {} // The content of child routes will be rendered here
///     }
/// }
///
/// #[component]
/// fn Child() -> Element {
///     rsx! {
///         p {
///             "Child"
///         }
///     }
/// }
///
/// # #[component]
/// # fn App() -> Element {
/// #     rsx! {
/// #         dioxus_router::components::HistoryProvider {
/// #             history:  move |_| std::rc::Rc::new(dioxus_history::MemoryHistory::with_initial_path(Route::Child {}.to_string())) as std::rc::Rc<dyn dioxus_history::History>,
/// #             Router::<Route> {}
/// #         }
/// #     }
/// # }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # vdom.rebuild_in_place();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><p>Child</p>");
/// ```
pub fn Outlet<R: Routable + Clone>() -> Element {
    OutletContext::<R>::render()
}

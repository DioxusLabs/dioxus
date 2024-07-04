use crate::prelude::*;
use crate::utils::use_router_internal::use_router_internal;

/// A hook that provides access to information about the current routing location.
///
/// > The Routable macro will define a version of this hook with an explicit type.
///
/// # Panic
/// - When the calling component is not nested within a [`Router`] component.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::{prelude::*};
///
/// #[derive(Clone, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
///
/// #[component]
/// fn App() -> Element {
///     rsx! {
///         h1 { "App" }
///         Router::<Route> {}
///     }
/// }
///
/// #[component]
/// fn Index() -> Element {
///     let path: Route = use_route();
///     rsx! {
///         h2 { "Current Path" }
///         p { "{path}" }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # vdom.rebuild_in_place();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><h2>Current Path</h2><p>/</p>")
/// ```
#[doc(alias = "use_url")]
#[must_use]
pub fn use_route<R: Routable + Clone>() -> R {
    match use_router_internal() {
        Some(r) => r.current(),
        None => {
            panic!("`use_route` must be called in a descendant of a Router component")
        }
    }
}

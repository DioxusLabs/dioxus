use dioxus::prelude::ScopeState;

use crate::prelude::*;
use crate::utils::use_router_internal::use_router_internal;

/// A hook that provides access to information about the current routing location.
///
/// > The Routable macro will define a version of this hook with an explicit type.
///
/// # Return values
/// - None, when not called inside a [`Link`] component.
/// - Otherwise the current route.
///
/// # Panic
/// - When the calling component is not nested within a [`Link`] component during a debug build.
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
/// fn App(cx: Scope) -> Element {
///     render! {
///         h1 { "App" }
///         Router::<Route> {}
///     }
/// }
///
/// #[component]
/// fn Index(cx: Scope) -> Element {
///     let path = use_route(&cx).unwrap();
///     render! {
///         h2 { "Current Path" }
///         p { "{path}" }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><h2>Current Path</h2><p>/</p>")
/// ```
pub fn use_route<R: Routable + Clone>(cx: &ScopeState) -> Option<R> {
    match use_router_internal(cx) {
        Some(r) => Some(r.current()),
        None => {
            #[cfg(debug_assertions)]
            panic!("`use_route` must have access to a parent router");
            #[allow(unreachable_code)]
            None
        }
    }
}

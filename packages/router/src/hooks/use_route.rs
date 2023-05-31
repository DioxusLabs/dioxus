use dioxus::prelude::ScopeState;

use crate::prelude::*;
use crate::utils::use_router_internal::use_router_internal;

/// A hook that provides access to information about the current routing location.
///
/// # Return values
/// - None, when not called inside a [`GenericRouter`] component.
/// - Otherwise the current route.
///
/// # Panic
/// - When the calling component is not nested within another component calling the [`use_router`]
///   hook, but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use serde::{Deserialize, Serialize};
/// # use dioxus_router::{history::*, prelude::*};
///
/// #[derive(Clone, Serialize, Deserialize, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
///
/// fn App(cx: Scope) -> Element {
///     render! {
///         h1 { "App" }
///         Router {}
///     }
/// }
///
/// #[inline_props]
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
pub fn use_generic_route<R: Routable + Clone>(cx: &ScopeState) -> Option<R> {
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

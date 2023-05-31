use dioxus::prelude::ScopeState;

use crate::{
    prelude::GenericRouterContext, routable::Routable,
    utils::use_router_internal::use_router_internal,
};

/// A hook that provides access to information about the router. The Router will define a version of this hook with an explicit type.
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::{history::*, prelude::*};
/// # use serde::{Deserialize, Serialize};
/// #[derive(Clone, Serialize, Deserialize, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
///     #[route("/:id")]
///     Dynamic { id: usize },
/// }
///
/// fn App(cx: Scope) -> Element {
///     render! {
///         Router {}
///     }
/// }
///
/// #[inline_props]
/// fn Index(cx: Scope) -> Element {
///     let router = use_router(&cx);
///
///     render! {
///         button {
///             onclick: move |_| { router.push(Route::Dynamic { id: 1234 }); },
///             "Go to /1234"
///         }
///     }
/// }
///
/// #[inline_props]
/// fn Dynamic(cx: Scope, id: usize) -> Element {
///     render! {
///         p {
///             "Current ID: {id}"
///         }
///     }
/// }
///
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// ```
pub fn use_generic_router<R: Routable + Clone>(cx: &ScopeState) -> &GenericRouterContext<R> {
    use_router_internal(cx)
        .as_ref()
        .expect("use_route must have access to a router")
}

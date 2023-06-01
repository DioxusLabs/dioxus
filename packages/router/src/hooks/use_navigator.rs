use dioxus::prelude::ScopeState;

use crate::{
    prelude::{GenericNavigator, GenericRouterContext},
    routable::Routable,
};

/// A hook that provides access to the navigator to change the router history. Unlike [`use_router`], this hook will not cause a rerender when the current route changes
///
/// > The Routable macro will define a version of this hook with an explicit type.
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
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
///     let navigator = use_navigator(&cx);
///
///     render! {
///         button {
///             onclick: move |_| { navigator.push(Route::Dynamic { id: 1234 }); },
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
pub fn use_generic_navigator<R: Routable + Clone>(cx: &ScopeState) -> &GenericNavigator<R> {
    &*cx.use_hook(|| {
        let router = cx
            .consume_context::<GenericRouterContext<R>>()
            .expect("Must be called in a descendant of a Router component");

        GenericNavigator(router)
    })
}

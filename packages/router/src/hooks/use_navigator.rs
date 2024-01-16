use dioxus_lib::prelude::{try_consume_context, use_hook};

use crate::prelude::{Navigator, RouterContext};

/// A hook that provides access to the navigator to change the router history. Unlike [`use_router`], this hook will not cause a rerender when the current route changes
///
/// > The Routable macro will define a version of this hook with an explicit type.
///
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// #[derive(Clone, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
///     #[route("/:id")]
///     Dynamic { id: usize },
/// }
///
/// #[component]
/// fn App() -> Element {
///     rsx! {
///         Router::<Route> {}
///     }
/// }
///
/// #[component]
/// fn Index() -> Element {
///     let navigator = use_navigator(&cx);
///
///     rsx! {
///         button {
///             onclick: move |_| { navigator.push(Route::Dynamic { id: 1234 }); },
///             "Go to /1234"
///         }
///     }
/// }
///
/// #[component]
/// fn Dynamic(id: usize) -> Element {
///     rsx! {
///         p {
///             "Current ID: {id}"
///         }
///     }
/// }
///
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// ```
#[must_use]
pub fn use_navigator() -> Navigator {
    use_hook(|| {
        let router = try_consume_context::<RouterContext>()
            .expect("Must be called in a descendant of a Router component");

        Navigator(router)
    })
}

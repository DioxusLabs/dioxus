use dioxus::prelude::{ScopeId, ScopeState};
use dioxus_router_core::Navigator;

use crate::{utils::use_router_internal::use_router_internal};

/// A hook that allows for programmatic navigation.
///
/// # Return values
/// - [`RouterError::NotInsideRouter`], when the calling component is not nested within another
///   component calling the [`use_router`] hook.
/// - Otherwise [`Ok`].
///
/// [`use_router`]: crate::hooks::use_router
///
/// # Panic
/// - When the calling component is not nested within another component calling the [`use_router`]
///   hook, but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// fn App(cx: Scope) -> Element {
///     let (state, _) = use_router(
///         &cx,
///         &|| RouterConfiguration {
///             synchronous: true, // asynchronicity not needed for doc test
///             ..Default::default()
///         },
///         &|| Segment::content(comp(Redirect)).fixed("content", comp(Content))
///     );
///
///     render! {
///         h1 { "App" }
///         Outlet { }
///     }
/// }
///
/// fn Redirect(cx: Scope) -> Element {
///     let nav = use_navigate(&cx)?;
///     nav.push("/content");
///     render! { () }
/// }
///
/// fn Content(cx: Scope) -> Element {
///     render! {
///         p { "Content" }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// #
/// # // first render with Redirect component
/// # let _ = vdom.rebuild();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1>");
/// #
/// # // second render with Content component
/// # let _ = vdom.rebuild();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><p>Content</p>");
/// ```
pub fn use_navigate(cx: &ScopeState) -> Option<Navigator<ScopeId>> {
    match use_router_internal(cx) {
        Some(r) => Some(r.sender.clone().into()),
        None => {
            #[cfg(debug_assertions)]
            panic!("`use_navigate` must have access to a parent router");
            #[allow(unreachable_code)]
            None
        }
    }
}

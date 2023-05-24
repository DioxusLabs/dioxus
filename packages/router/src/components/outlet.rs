use crate::{contexts::outlet::OutletContext, routable::Routable};
use dioxus::prelude::*;

/// An outlet for the current content.
///
/// Only works as descendant of a component calling [`use_router`], otherwise it will be inactive.
///
/// The [`Outlet`] is aware of how many [`Outlet`]s it is nested within. It will render the content
/// of the active route that is __exactly as deep__.
///
/// [`use_router`]: crate::hooks::use_router
///
/// # Panic
/// - When the [`Outlet`] is not nested within another component calling the [`use_router`] hook,
///   but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// fn App(cx: Scope) -> Element {
///     use_router(
///         &cx,
///         &|| RouterConfiguration {
///             synchronous: true, // asynchronicity not needed for doc test
///             ..Default::default()
///         },
///         &|| Segment::content(comp(Content))
///     );
///
///     render! {
///         h1 { "App" }
///         Outlet { } // The content component will be rendered here
///     }
/// }
///
/// fn Content(cx: Scope) -> Element {
///     render! {
///         p { "Content" }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><p>Content</p>");
/// ```
pub fn GenericOutlet<R: Routable + Clone>(cx: Scope) -> Element {
    OutletContext::render::<R>(cx)
}

use dioxus::prelude::ScopeState;

use crate::{routable::Routable, utils::use_router_internal::use_router_internal};

/// A hook that provides access to information about the current routing location.
///
/// # Return values
/// - [`RouterError::NotInsideRouter`], when the calling component is not nested within another
///   component calling the [`use_router`] hook.
/// - Otherwise [`Ok`].
///
/// # Important usage information
/// Make sure to [`drop`] the returned [`RwLockReadGuard`] when done rendering. Otherwise the router
/// will be frozen.
///
/// # Panic
/// - When the calling component is not nested within another component calling the [`use_router`]
///   hook, but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::{history::*, prelude::*};
/// fn App(cx: Scope) -> Element {
///     use_router(
///         &cx,
///         &|| RouterConfiguration {
///             synchronous: true, // asynchronicity not needed for doc test
///             history: Box::new(MemoryHistory::with_initial_path("/some/path").unwrap()),
///             ..Default::default()
///         },
///         &|| Segment::empty()
///     );
///
///     render! {
///         h1 { "App" }
///         Content { }
///     }
/// }
///
/// fn Content(cx: Scope) -> Element {
///     let state = use_route(&cx)?;
///     let path = state.path.clone();
///
///     render! {
///         h2 { "Current Path" }
///         p { "{path}" }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><h2>Current Path</h2><p>/some/path</p>")
/// ```
///
/// [`use_router`]: crate::hooks::use_router
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

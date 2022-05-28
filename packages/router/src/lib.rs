//! A router for dioxus.
//!
//! You are reading the API documentation, which describes the entirety of the routers API. If you
//! are looking for a guide-like documentation see the [router book].
//!
//!
//! # Example
//! ```rust
//! use dioxus::prelude::*;
//! # use dioxus::router::history::MemoryHistoryProvider;
//!
//! fn App(cx: Scope) -> Element {
//!     // declare the routes of the app
//!     let routes = use_segment(&cx, || {
//!         Segment::new()
//!             .index(RcComponent(Index)) // when the path is '/'
//!             .fixed("other", Route::new(RcComponent(Other))) // when the path is `/other`
//!     });
//!
//!     cx.render(rsx! {
//!         // render the router and give it the routes
//!         Router {
//!             routes: routes.clone(),
//!             # // needed for the test at the end
//!             # init_only: true,
//!             // give the router a place to render the content
//!             Outlet { }
//!         }
//!     })
//! }
//!
//! fn Index(cx: Scope) -> Element {
//!     cx.render(rsx! {
//!         h1 { "Example" }
//!     })
//! }
//!
//! fn Other(cx: Scope) -> Element {
//!     cx.render(rsx! {
//!         p { "Some content" }
//!     })
//! }
//! #
//! # let mut vdom = VirtualDom::new(App);
//! # vdom.rebuild();
//! # assert_eq!("<h1>Example</h1>", dioxus::ssr::render_vdom(&vdom));
//! ```
//!
//! [router book]: https://dioxuslabs.com/router/guide

#![deny(missing_docs)]

/// The components of the router.
pub mod components {
    mod link;
    pub use link::*;

    mod history_buttons;
    pub use history_buttons::*;

    mod outlet;
    pub use outlet::*;

    mod router;
    pub use router::*;
}

/// All contexts of the router.
///
/// Contexts are deliberately not exposed to users. The public API consists of components and hooks.
mod contexts {
    mod outlet;
    pub(crate) use outlet::*;

    mod router;
    pub(crate) use router::*;
}

/// Helper functions used within the router.
mod helpers;

/// Implementations of the navigation history.
pub mod history;

/// The hooks of the router.
pub mod hooks {
    mod use_navigate;
    pub use use_navigate::*;

    mod use_route;
    pub use use_route::*;

    mod use_segment;
    pub use use_segment::*;
}

/// Navigation information.
pub mod navigation;

/// Fallback path for failed external navigation. See [`NtExternal`].
///
/// [`NtExternal`]: navigation::NavigationTarget::NtExternal
pub const PATH_FOR_EXTERNAL_NAVIGATION_FAILURE: &str = "dioxus-router-external-navigation-failure";

/// Fallback path for failed named navigation. See [`NtName`].
///
/// [`NtName`]: navigation::NavigationTarget::NtName
pub const PATH_FOR_NAMED_NAVIGATION_FAILURE: &str = "dioxus-router-named-navigation-failure";

/// Reexports of commonly used elements.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::hooks::*;
    pub use crate::navigation::{NavigationTarget::*, Query::*, *};
    pub use crate::route_definition::{RouteContent::*, *};
    pub use crate::state::RouterState;
}

/// Application-defined routing information.
pub mod route_definition {
    mod content;
    pub use content::*;

    mod dynamic;
    pub(crate) use dynamic::*;

    mod parameter;
    pub use parameter::*;

    mod route;
    pub use route::*;

    mod segment;
    pub use segment::*;
}

/// The core of the router.
mod service;

/// Information about the current route.
pub mod state;

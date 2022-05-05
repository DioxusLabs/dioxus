//! A router for dioxus.
//!
//! You are reading the API documentation, which describes the elements of the public API. If you
//! are more interested in a usage-oriented documentation see the [router book].
//!
//! [router book]: https://dioxuslabs.com/router/guide

#![deny(missing_docs)]

/// All components of the router.
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

mod helpers;

/// Types relating to the navigation history.
pub mod history;

/// All hooks of the router.
pub mod hooks {
    mod use_navigate;
    pub use use_navigate::*;

    mod use_route;
    pub use use_route::*;
}

/// Types relating to navigation.
pub mod navigation;

/// Fallback path for failed external navigation. See [`NtExternal`].
///
/// [`NtExternal`]: navigation::NavigationTarget::NtExternal
pub const PATH_FOR_EXTERNAL_NAVIGATION_FAILURE: &'static str =
    "dioxus-router-external-navigation-failure";

/// Fallback path for failed named navigation. See [`NtName`].
///
/// [`NtName`]: navigation::NavigationTarget::NtName
pub const PATH_FOR_NAMED_NAVIGATION_FAILURE: &'static str =
    "dioxus-router-named-navigation-failure";

/// Reexports of commonly used elements.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::hooks::*;
    pub use crate::navigation::{NavigationTarget::*, Query::*, *};
    pub use crate::route_definition::{DynamicRoute::*, RouteContent::*, *};
    pub use crate::state::RouterState;
}

/// Types to tell the router what to render.
pub mod route_definition;

mod service;

/// Types providing access to the internal state of the router.
pub mod state;

#![doc = include_str!("../README.md")]
//!
//! You are reading the API documentation, which describes the entirety of the routers API. If you
//! are looking for a guide-like documentation see the [router book](https://dioxuslabs.com/router/guide).

#![deny(missing_docs)]

/// The components of the router.
pub mod components {
    mod fallback_named_navigation;
    pub(crate) use fallback_named_navigation::*;

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

/// Fallback path for failed external navigation. See [`ExternalTarget`].
///
/// [`ExternalTarget`]: navigation::NavigationTarget::ExternalTarget
pub const PATH_FOR_EXTERNAL_NAVIGATION_FAILURE: &str = "dioxus-router-external-navigation-failure";

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

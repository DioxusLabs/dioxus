#![doc = include_str!("../README.md")]
//!
//! You are reading the API documentation, which describes the entirety of the routers API. If you
//! are looking for a guide-like documentation see the [router book](https://dioxuslabs.com/router/guide).

#![deny(missing_docs)]

/// The components of the router.
pub mod components {
    mod fallback_defaults;
    pub(crate) use fallback_defaults::*;

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

/// Reexports of commonly used elements.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::hooks::*;
    pub use crate::names::*;
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

/// Route names used by the router itself.
pub mod names {
    /// The name the router will automatically assign to the index route of the root segment.
    pub struct RootIndex;

    /// Indicator for failed external navigation.
    ///
    /// Will be added to the names list when the router is handling a failed external navigation.
    ///
    /// **IMPORTANT:** This name cannot be navigated to.
    pub struct FallbackExternalNavigation;

    /// Indicator for a failed named navigation.
    ///
    /// Will be added to the names list when the router is handling a failed named navigation.
    ///
    /// **IMPORTANT:** This name cannot be navigated to.
    pub struct FallbackNamedNavigation;

    // CAUTION!!
    // =========
    // When adding new names, make sure to check for them when extracting all named routes.
    // This is currently done by `construct_named_targets` in `service.rs`.

    // TODO: All TypeIds of router names could be added to an array here, which would be easier to
    //       maintain. However, this currently results in a compiler error:
    //       "`TypeId::of` is not yet stable as a const fn"
    //
    // const ROUTER_NAME_TYPE_IDS: [TypeId; 3] = [
    //     TypeId::of::<RootIndex>(),
    //     TypeId::of::<FallbackExternalNavigation>(),
    //     TypeId::of::<FallbackNamedNavigation>(),
    // ];
}

/// The core of the router.
mod service;

/// Information about the current route.
pub mod state;

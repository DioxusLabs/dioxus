#![deny(missing_docs)]

//! A router for dioxus.

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

/// When the router was given an [`NtExternal`] navigation target and the [`HistoryProvider`]
/// doesn't support external navigation targets, this is the path the router will navigate to.
///
/// The external url will be provided as a query string parameter named `url`.
///
/// [`HistoryProvider`]: history::HistoryProvider
/// [`NtExternal`]: navigation::NavigationTarget::NtExternal
pub const EXTERNAL_NAVIGATION_FAILURE_PATH: &'static str =
    "dioxus-router-external-navigation-failure";

mod helpers;

pub mod history;

/// All hooks of the router.
pub mod hooks {
    mod use_navigate;
    pub use use_navigate::*;

    mod use_route;
    pub use use_route::*;
}

/// When the router was given an [`NtName`] navigation target and no route with the provided path is
/// known, this is the path the router will navigate to.
///
/// [`NtName`]: navigation::NavigationTarget::NtName
pub const NAMED_NAVIGATION_FAILURE_PATH: &'static str = "dioxus-router-named-navigation-failure";

pub mod navigation;

/// Reexports of commonly used elements.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::hooks::*;
    pub use crate::navigation::{NavigationTarget::*, Query::*, *};
    pub use crate::route_definition::{DynamicRoute::*, RouteContent::*, *};
    pub use crate::state::*;
}

pub mod route_definition;

mod service;

pub mod state;

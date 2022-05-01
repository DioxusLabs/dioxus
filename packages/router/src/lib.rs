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

mod helpers;

pub mod history;

/// All hooks of the router.
pub mod hooks {
    mod use_navigate;
    pub use use_navigate::*;

    mod use_route;
    pub use use_route::*;
}

pub mod navigation;

/// Reexports of commonly used elements.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::hooks::*;
    pub use crate::navigation::{NavigationTarget::*, *};
    pub use crate::route_definition::*;
    pub use crate::state::*;
}

pub mod route_definition;

mod service;

pub mod state;

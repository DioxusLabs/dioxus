#![doc = include_str!("../README.md")]
// cannot use forbid, because props derive macro generates #[allow(missing_docs)]
#![deny(missing_docs)]
#![allow(non_snake_case)]

pub mod navigation;
pub mod routable;

/// Components interacting with the router.
pub mod components {
    pub(crate) mod default_errors;

    mod history_buttons;
    pub use history_buttons::*;

    mod link;
    pub use link::*;

    mod outlet;
    pub use outlet::*;

    mod router;
    pub use router::*;
}

mod contexts {
    pub(crate) mod outlet;
    pub(crate) mod router;
    pub use router::*;
}

mod router_cfg;

pub mod history;

/// Hooks for interacting with the router in components.
pub mod hooks {
    mod use_router;
    pub use use_router::*;

    mod use_route;
    pub use use_route::*;
}

/// A collection of useful items most applications might need.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::contexts::*;
    pub use crate::hooks::*;
    pub use crate::router_cfg::RouterConfiguration;
    pub use dioxus_router_macro::Routable;
}

mod utils {
    pub(crate) mod use_router_internal;
}

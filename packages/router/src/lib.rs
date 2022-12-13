/// Components interacting with the router.
#[deny(missing_docs)]
pub mod components {
    pub(crate) mod default_errors;

    mod history_buttons;
    pub use history_buttons::*;

    mod link;
    pub use link::*;

    mod outlet;
    pub use outlet::*;
}

mod contexts {
    pub(crate) mod router;
}

#[forbid(missing_docs)]
mod error;
pub use error::RouterError;

pub mod history {
    pub use dioxus_router_core::history::*;
}

/// Hooks for interacting with the router in components.
#[forbid(missing_docs)]
pub mod hooks {
    mod use_navigate;
    pub use use_navigate::*;

    mod use_router;
    pub use use_router::*;

    mod use_route;
    pub use use_route::*;
}

/// A collection of useful items most applications might need.
#[forbid(missing_docs)]
pub mod prelude {
    pub use dioxus_router_core::prelude::*;

    pub use crate::components::*;
    pub use crate::hooks::*;

    /// Wrap a [`Component`](dioxus::core::Component) inside a [`ContentAtom`].
    ///
    /// This is purely a convenience function.
    #[must_use]
    pub fn comp(component: dioxus::core::Component) -> ContentAtom<dioxus::core::Component> {
        ContentAtom(component)
    }
}

mod utils {
    pub(crate) mod use_router_internal;
}

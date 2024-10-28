#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
// cannot use forbid, because props derive macro generates #[allow(missing_docs)]
#![deny(missing_docs)]
#![allow(non_snake_case)]

pub mod navigation;
pub mod routable;

/// Components interacting with the router.
pub mod components {
    mod default_errors;
    pub use default_errors::*;

    mod history_buttons;
    pub use history_buttons::*;

    mod link;
    pub use link::*;

    mod outlet;
    pub use outlet::*;

    mod router;
    pub use router::*;

    mod history_provider;
    pub use history_provider::*;

    #[doc(hidden)]
    pub mod child_router;
}

mod contexts {
    pub(crate) mod navigator;
    pub(crate) mod outlet;
    pub(crate) mod router;
    pub use navigator::*;
    pub(crate) use router::*;
    pub use router::{root_router, RouterContext};
}

mod router_cfg;

/// Hooks for interacting with the router in components.
pub mod hooks {
    mod use_router;
    pub use use_router::*;

    mod use_route;
    pub use use_route::*;

    mod use_navigator;
    pub use use_navigator::*;
}

pub use hooks::router;

/// A collection of useful items most applications might need.
pub mod prelude {
    pub use crate::components::{
        GoBackButton, GoForwardButton, HistoryButtonProps, Link, LinkProps, Outlet, Router,
        RouterProps,
    };
    pub use crate::contexts::*;
    pub use crate::hooks::*;
    pub use crate::navigation::*;
    pub use crate::routable::*;
    pub use crate::router_cfg::RouterConfig;
    pub use dioxus_router_macro::Routable;

    #[doc(hidden)]
    /// A component with props used in the macro
    pub trait HasProps {
        /// The props type of the component.
        type Props;
    }

    impl<P> HasProps for dioxus_lib::prelude::Component<P> {
        type Props = P;
    }
}

mod utils {
    pub(crate) mod use_router_internal;
}

#[doc(hidden)]
pub mod exports {
    pub use urlencoding;
}

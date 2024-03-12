#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
// cannot use forbid, because props derive macro generates #[allow(missing_docs)]
#![deny(missing_docs)]
#![allow(non_snake_case)]

pub mod navigation;
pub mod routable;

#[cfg(feature = "ssr")]
pub mod incremental;

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
}

mod contexts {
    pub(crate) mod navigator;
    pub(crate) mod outlet;
    pub(crate) mod router;
    pub use navigator::*;
    pub(crate) use router::*;
}

mod router_cfg;

mod history;

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
    pub use crate::components::*;
    pub use crate::contexts::*;
    pub use crate::history::*;
    pub use crate::hooks::*;
    pub use crate::navigation::*;
    pub use crate::routable::*;
    pub use crate::router_cfg::RouterConfig;
    pub use dioxus_router_macro::Routable;

    #[cfg(feature = "ssr")]
    pub use crate::incremental::*;
    #[cfg(feature = "ssr")]
    pub use dioxus_ssr::incremental::*;

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

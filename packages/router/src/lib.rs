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
    #[cfg(feature = "html")]
    mod default_errors;
    #[cfg(feature = "html")]
    pub use default_errors::*;

    #[cfg(feature = "html")]
    mod history_buttons;
    #[cfg(feature = "html")]
    pub use history_buttons::*;

    #[cfg(feature = "html")]
    mod link;
    #[cfg(feature = "html")]
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
    pub use outlet::{use_outlet_context, OutletContext};
    pub(crate) mod router;
    pub use navigator::*;
    pub(crate) use router::*;
    pub use router::{root_router, GenericRouterContext, ParseRouteError, RouterContext};
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

#[cfg(feature = "html")]
pub use crate::components::{GoBackButton, GoForwardButton, HistoryButtonProps, Link, LinkProps};
pub use crate::components::{Outlet, Router, RouterProps};
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

impl<P> HasProps for dioxus_core::Component<P> {
    type Props = P;
}

mod utils {
    pub(crate) mod use_router_internal;
}

#[doc(hidden)]
pub mod exports {
    pub use crate::query_sets::*;
    pub use percent_encoding;
}

pub(crate) mod query_sets {
    //! Url percent encode sets defined [here](https://url.spec.whatwg.org/#percent-encoded-bytes)

    use percent_encoding::AsciiSet;

    /// The ASCII set that must be escaped in query strings.
    pub const QUERY_ASCII_SET: &AsciiSet = &percent_encoding::CONTROLS
        .add(b' ')
        .add(b'"')
        .add(b'#')
        .add(b'<')
        .add(b'>');

    /// The ASCII set that must be escaped in path segments.
    pub const PATH_ASCII_SET: &AsciiSet = &QUERY_ASCII_SET
        .add(b'?')
        .add(b'^')
        .add(b'`')
        .add(b'{')
        .add(b'}');

    /// The ASCII set that must be escaped in hash fragments.
    pub const FRAGMENT_ASCII_SET: &AsciiSet = &percent_encoding::CONTROLS
        .add(b' ')
        .add(b'"')
        .add(b'<')
        .add(b'>')
        .add(b'`');
}

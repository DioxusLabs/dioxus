#![doc = include_str!("../README.md")]
#![forbid(missing_docs)]

pub mod history;

mod name;
pub use name::*;

pub mod navigation;

mod navigator;
pub use navigator::*;

mod outlet;
pub use outlet::*;

/// Types for defining the available routes.
pub mod routes {
    mod atom;
    pub use atom::*;

    mod content;
    pub use content::*;

    mod matcher;
    pub use matcher::*;

    mod route;
    pub use route::*;

    mod segment;
    pub use segment::*;

    mod parameter_route;
    pub use parameter_route::*;
}

mod service;
pub use service::*;

mod state;
pub use state::*;

mod utils {
    mod name;
    pub use name::*;

    mod route;
    pub use route::*;

    mod sitemap;
    pub use sitemap::*;

    mod target;
    pub use target::*;
}

/// A collection of useful types most applications might need.
pub mod prelude {
    pub use crate::name::*;
    pub use crate::navigation::*;
    pub use crate::routes::*;

    /// An external navigation failure.
    ///
    /// These occur when the router tries to navigate to a [`NavigationTarget::External`] and the
    /// [`HistoryProvider`](crate::history::HistoryProvider) doesn't support that.
    pub struct FailureExternalNavigation;

    /// A named navigation failure.
    ///
    /// These occur when the router tries to navigate to a [`NavigationTarget::Named`] and the
    /// specified [`Name`](crate::Name) is unknown, or a parameter is missing.
    pub struct FailureNamedNavigation;

    /// A redirection limit breach.
    ///
    /// These occur when the router tries to navigate to any target, but encounters 25 consecutive
    /// redirects.
    pub struct FailureRedirectionLimit;

    /// The [`Name`](crate::Name) equivalent of `/`.
    pub struct RootIndex;
}

#![doc = include_str!("../README.md")]
#![forbid(missing_docs)]

pub mod history;

mod router;
pub use router::*;

pub mod navigation;

mod navigator;
pub use navigator::*;

mod service;
pub use service::*;

mod state;
pub use state::*;

mod utils {
    mod sitemap;
    pub use sitemap::*;
}

/// A collection of useful types most applications might need.
pub mod prelude {
    pub use crate::navigation::*;

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

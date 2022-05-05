//! A trait that defines methods used by the router for history and navigation and default
//! implementations.

use std::sync::Arc;

#[cfg(feature = "web")]
mod browser_hash;
#[cfg(feature = "web")]
pub use browser_hash::*;

#[cfg(feature = "web")]
mod browser_path;
#[cfg(feature = "web")]
pub use browser_path::*;

mod controlled;
pub use controlled::*;

mod memory;
pub use memory::*;

#[allow(rustdoc::private_intra_doc_links)]
/// Several operations used by the [router service](crate::service::RouterService).
///
/// **INFO:** The struct referenced in the summary of this trait isn't `pub`. To look at its
/// documentation either look at the source code or build the documentation with
/// `--document-private-items`.
pub trait HistoryProvider {
    /// Provides the [`HistoryProvider`] with a way to trigger a routing update.
    ///
    /// Some [`HistoryProvider`]s may receive updates from outside the router and need to inform it
    /// about the new location. To do that, they can call the provided callback.
    #[allow(unused)]
    fn foreign_navigation_handler(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {}

    /// Get the current path.
    fn current_path(&self) -> String;
    /// Get a prefix for `href`s.
    fn current_prefix(&self) -> String {
        String::new()
    }
    /// Get the current query string.
    fn current_query(&self) -> Option<String>;

    /// Check if there is a prior path that can be navigated back to.
    ///
    /// If unknown return `true` and do nothing when `go_back` is called.
    fn can_go_back(&self) -> bool {
        true
    }
    /// Check if there is a future path that can be navigated forward to.
    ///
    /// If unknown return `true` and do nothing when `go_forward` is called.
    fn can_go_forward(&self) -> bool {
        true
    }

    /// Navigate to the last active path.
    fn go_back(&mut self);
    /// Navigate to the next path. The inverse function of [`HistoryProvider::go_back`].
    fn go_forward(&mut self);

    /// Push a new path onto the history.
    ///
    /// Only works for internal targets.
    fn push(&mut self, path: String);
    /// Replace the current path with a new one.
    ///
    /// Only works for internal targets.
    fn replace(&mut self, path: String);

    /// Whether the provider can handle external targets.
    fn can_external(&self) -> bool {
        false
    }
    /// Go to an external target.
    ///
    /// May be called even if [`Self::can_external`] returns [`false`].
    #[allow(unused)]
    fn external(&self, url: String) {}
}

#[derive(serde::Deserialize, serde::Serialize)]
struct ScrollPosition {
    x: i32,
    y: i32,
}

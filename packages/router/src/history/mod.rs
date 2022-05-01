//! A trait that defines methods used by the router for history and navigation and default
//! implementations.

mod memory;
pub use memory::*;

#[allow(rustdoc::private_intra_doc_links)]
/// Several operations used by the [router service](crate::service::RouterService).
///
/// **INFO:** The struct referenced in the summary of this trait isn't `pub`. To look at its
/// documentation either look at the source code or build the documentation with
/// `--document-private-items`.
pub trait HistoryProvider {
    /// Get the current path.
    fn current_path(&self) -> &str;

    /// Check if there is a prior path that can be navigated back to.
    fn can_go_back(&self) -> bool;
    /// Check if there is a future path that can be navigated forward to.
    fn can_go_forward(&self) -> bool;

    /// Navigate to the last active path.
    fn go_back(&mut self);
    /// Navigate to the next path. The inverse function of [`HistoryProvider::go_back`].
    fn go_forward(&mut self);

    /// Push a new path onto the history.
    fn push(&mut self, path: String);

    /// Replace the current path with a new one.
    fn replace(&mut self, path: String);
}

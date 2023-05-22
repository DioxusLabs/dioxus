//! History Integration
//!
//! dioxus-router-core relies on so-called [`HistoryProvider`]s to store the current URL, and possibly a
//! history (i.e. a browsers back button) and future (i.e. a browsers forward button).
//!
//! To integrate dioxus-router-core with a any type of history, all you have to do is implement the
//! [`HistoryProvider`] trait. dioxus-router-core also comes with some (for now one) default implementations.

use std::sync::Arc;

mod memory;
pub use memory::*;

#[cfg(feature = "web")]
mod web;
#[cfg(feature = "web")]
pub use web::*;

#[cfg(feature = "web")]
mod web_hash;
#[cfg(feature = "web")]
pub use web_hash::*;

use crate::routable::Routable;

#[cfg(feature = "web")]
pub(crate) mod web_scroll;

/// An integration with some kind of navigation history.
///
/// Depending on your use case, your implementation may deviate from the described procedure. This
/// is fine, as long as both `current_path` and `current_query` match the described format.
///
/// However, you should document all deviations. Also, make sure the navigation is user-friendly.
/// The described behaviors are designed to mimic a web browser, which most users should already
/// know. Deviations might confuse them.
pub trait HistoryProvider<R: Routable> {
    /// Get the path of the current URL.
    ///
    /// **Must start** with `/`. **Must _not_ contain** the prefix.
    ///
    /// ```rust
    /// # use dioxus_router_core::history::{HistoryProvider, MemoryHistory};
    /// let mut history = MemoryHistory::default();
    /// assert_eq!(history.current_path(), "/");
    ///
    /// history.push(String::from("/path"));
    /// assert_eq!(history.current_path(), "/path");
    /// ```
    #[must_use]
    fn current_route(&self) -> &R;

    /// Get the current path prefix of the URL.
    ///
    /// Not all [`HistoryProvider`]s need a prefix feature. It is meant for environments where a
    /// dioxus-router-core-routed application is not running on `/`. The [`HistoryProvider`] is responsible
    /// for removing the prefix from the dioxus-router-core-internal path, and also for adding it back in
    /// during navigation. This functions value is only used for creating `href`s (e.g. for SSR or
    /// display (but not navigation) in a web app).
    fn current_prefix(&self) -> Option<String> {
        None
    }

    /// Check whether there is a previous page to navigate back to.
    ///
    /// If a [`HistoryProvider`] cannot know this, it should return [`true`].
    ///
    /// ```rust
    /// # use dioxus_router_core::history::{HistoryProvider, MemoryHistory};
    /// let mut history = MemoryHistory::default();
    /// assert_eq!(history.can_go_back(), false);
    ///
    /// history.push(String::from("/some-other-page"));
    /// assert_eq!(history.can_go_back(), true);
    /// ```
    #[must_use]
    fn can_go_back(&self) -> bool {
        true
    }

    /// Go back to a previous page.
    ///
    /// If a [`HistoryProvider`] cannot go to a previous page, it should do nothing. This method
    /// might be called, even if `can_go_back` returns [`false`].
    ///
    /// ```rust
    /// # use dioxus_router_core::history::{HistoryProvider, MemoryHistory};
    /// let mut history = MemoryHistory::default();
    /// assert_eq!(history.current_path(), "/");
    ///
    /// history.go_back();
    /// assert_eq!(history.current_path(), "/");
    ///
    /// history.push(String::from("/some-other-page"));
    /// assert_eq!(history.current_path(), "/some-other-page");
    ///
    /// history.go_back();
    /// assert_eq!(history.current_path(), "/");
    /// ```
    fn go_back(&mut self);

    /// Check whether there is a future page to navigate forward to.
    ///
    /// If a [`HistoryProvider`] cannot know this, it should return [`true`].
    ///
    /// ```rust
    /// # use dioxus_router_core::history::{HistoryProvider, MemoryHistory};
    /// let mut history = MemoryHistory::default();
    /// assert_eq!(history.can_go_forward(), false);
    ///
    /// history.push(String::from("/some-other-page"));
    /// assert_eq!(history.can_go_forward(), false);
    ///
    /// history.go_back();
    /// assert_eq!(history.can_go_forward(), true);
    /// ```
    #[must_use]
    fn can_go_forward(&self) -> bool {
        true
    }

    /// Go forward to a future page.
    ///
    /// If a [`HistoryProvider`] cannot go to a previous page, it should do nothing. This method
    /// might be called, even if `can_go_forward` returns [`false`].
    ///
    /// ```rust
    /// # use dioxus_router_core::history::{HistoryProvider, MemoryHistory};
    /// let mut history = MemoryHistory::default();
    /// history.push(String::from("/some-other-page"));
    /// assert_eq!(history.current_path(), "/some-other-page");
    ///
    /// history.go_back();
    /// assert_eq!(history.current_path(), "/");
    ///
    /// history.go_forward();
    /// assert_eq!(history.current_path(), "/some-other-page");
    /// ```
    fn go_forward(&mut self);

    /// Go to another page.
    ///
    /// This should do three things:
    /// 1. Merge the current URL with the `path` parameter (which may also include a query part).
    /// 2. Remove the previous URL to the navigation history.
    /// 3. Clear the navigation future.
    ///
    /// ```rust
    /// # use dioxus_router_core::history::{HistoryProvider, MemoryHistory};
    /// let mut history = MemoryHistory::default();
    /// assert_eq!(history.current_path(), "/");
    ///
    /// history.push(String::from("/some-other-page"));
    /// assert_eq!(history.current_path(), "/some-other-page");
    /// assert!(history.can_go_back());
    /// ```
    fn push(&mut self, route: R);

    /// Replace the current page with another one.
    ///
    /// This should merge the current URL with the `path` parameter (which may also include a query
    /// part). In contrast to the `push` function, the navigation history and future should stay
    /// untouched.
    ///
    /// ```rust
    /// # use dioxus_router_core::history::{HistoryProvider, MemoryHistory};
    /// let mut history = MemoryHistory::default();
    /// assert_eq!(history.current_path(), "/");
    ///
    /// history.replace(String::from("/some-other-page"));
    /// assert_eq!(history.current_path(), "/some-other-page");
    /// assert!(!history.can_go_back());
    /// ```
    fn replace(&mut self, path: R);

    /// Navigate to an external URL.
    ///
    /// This should navigate to an external URL, which isn't controlled by the router. If a
    /// [`HistoryProvider`] cannot do that, it should return [`false`], otherwise [`true`].
    ///
    /// Returning [`false`] will cause the router to handle the external navigation failure.
    #[allow(unused_variables)]
    fn external(&mut self, url: String) -> bool {
        false
    }

    /// Provide the [`HistoryProvider`] with an update callback.
    ///
    /// Some [`HistoryProvider`]s may receive URL updates from outside the router. When such
    /// updates are received, they should call `callback`, which will cause the router to update.
    #[allow(unused_variables)]
    fn updater(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {}
}

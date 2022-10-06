use std::{fmt::Debug, sync::Arc};

#[cfg(feature = "web")]
mod web_hash;
#[cfg(feature = "web")]
pub use web_hash::*;

#[cfg(feature = "web")]
mod web;
#[cfg(feature = "web")]
pub use web::*;

mod controlled;
pub use controlled::*;

mod memory;
pub use memory::*;

/// A trait that lets the router access the navigation history.
///
/// Provided implementations:
/// - [`MemoryHistory`] implements a history entirely in memory.
/// - [`WebHistory`] hooks up to the browsers history and URL.
/// - [`WebHashHistory`] hooks up to the browsers history, but stores the actual path
///   and query in the fragment of the browsers URL.
/// - [`HistoryController`] and [`ControlledHistory`] share an other [`HistoryProvider`] internally.
///   The [`HistoryController`] can be used to control the router from outside the VDOM.
pub trait HistoryProvider
where
    Self: Debug,
{
    /// Provides the [`HistoryProvider`] with a way to trigger a routing update.
    ///
    /// Some [`HistoryProvider`]s may receive updates from outside the router and need to inform it
    /// about the new location. To do that, they can call the provided `callback`.
    #[allow(unused)]
    fn foreign_navigation_handler(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {}

    /// Get the current path.
    #[must_use]
    fn current_path(&self) -> String;
    /// Get a prefix for `href`s.
    #[must_use]
    fn current_prefix(&self) -> String {
        String::new()
    }
    /// Get the current query string.
    #[must_use]
    fn current_query(&self) -> Option<String>;

    /// Check if there is a prior path that can be navigated back to.
    ///
    /// If unknown return [`true`] and do nothing when [`HistoryProvider::go_back`] is called.
    #[must_use]
    fn can_go_back(&self) -> bool {
        true
    }
    /// Check if there is a future path that can be navigated forward to.
    ///
    /// If unknown return [`true`] and do nothing when [`HistoryProvider::go_forward`] is called.
    #[must_use]
    fn can_go_forward(&self) -> bool {
        true
    }

    /// Navigate to the last active path.
    ///
    /// May be called even if [`HistoryProvider::can_go_back`] returns [`false`].
    fn go_back(&mut self);
    /// Navigate to the next path. The inverse function of [`HistoryProvider::go_back`].
    ///
    /// May be called even if [`HistoryProvider::can_go_forward`] returns [`false`].
    fn go_forward(&mut self);

    /// Push a new path onto the history.
    ///
    /// Only called for internal targets.
    fn push(&mut self, path: String);
    /// Replace the current path with a new one.
    ///
    /// Only called for internal targets.
    fn replace(&mut self, path: String);

    /// Whether the provider can handle external targets.
    #[must_use]
    fn can_external(&self) -> bool {
        false
    }
    /// Go to an external target.
    ///
    /// Only called if [`HistoryProvider::can_external`] returns [`true`].
    #[allow(unused)]
    fn external(&self, url: String) {}
}

/// The position the browser is scrolled to.
///
/// Used to restore it when navigating through history, by both [`BrowserPathHistoryProvider`] and
/// [`BrowserHashHistoryProvider`].
#[cfg(feature = "web")]
#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
struct ScrollPosition {
    x: f64,
    y: f64,
}

/// Replace the current history entry with an equivalent one, but with an updated scroll position.
///
/// Used by both [`BrowserPathHistoryProvider`] and [`BrowserHashHistoryProvider`].
#[cfg(feature = "web")]
fn update_history_with_scroll(window: &web_sys::Window, history: &web_sys::History) {
    use log::error;

    // get position
    let position = serde_wasm_bindgen::to_value(&ScrollPosition {
        x: window.scroll_x().unwrap_or_default(),
        y: window.scroll_y().unwrap_or_default(),
    })
    .unwrap();

    // replace in history
    if let Err(e) = history.replace_state(&position, "") {
        error!("failed to update scroll position in history: {e:?}");
    }
}

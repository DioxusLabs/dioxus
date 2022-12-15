use gloo::{
    history::{BrowserHistory, History, HistoryListener},
    utils::window,
};
use log::error;
use web_sys::Window;

use super::HistoryProvider;

/// A [`HistoryProvider`] that integrates with a browser via the [History API].
///
/// # Prefix
/// This [`HistoryProvider`] supports a prefix, which can be used for web apps that aren't located
/// at the root of their domain.
///
/// Application developers are responsible for ensuring that right after the prefix comes a `/`. If
/// that is not the case, this [`HistoryProvider`] will replace the first character after the prefix
/// with one.
///
/// Application developers are responsible for not rendering the router if the prefix is not present
/// in the URL. Otherwise, if a router navigation is triggered, the prefix will be added.
///
/// [History API]: https://developer.mozilla.org/en-US/docs/Web/API/History_API
pub struct WebHistory {
    history: BrowserHistory,
    listener_navigation: Option<HistoryListener>,
    prefix: Option<String>,
    window: Window,
}

impl WebHistory {
    /// Create a new [`WebHistory`] with a prefix.
    #[must_use]
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: Some(prefix.into()),
            ..Default::default()
        }
    }
}

impl Default for WebHistory {
    fn default() -> Self {
        Self {
            history: BrowserHistory::new(),
            listener_navigation: None,
            prefix: None,
            window: window(),
        }
    }
}

impl HistoryProvider for WebHistory {
    fn current_path(&self) -> String {
        let location = self.history.location();
        let path = location.path();

        match &self.prefix {
            None => path.to_string(),
            Some(prefix) => path
                .starts_with(prefix)
                .then(|| path.split_at(prefix.len()).1)
                .unwrap_or("/")
                .to_string(),
        }
    }

    fn current_query(&self) -> Option<String> {
        let location = self.history.location();
        let query = location.query_str();

        if query.is_empty() {
            None
        } else {
            Some(query.to_string())
        }
    }

    fn current_prefix(&self) -> Option<String> {
        self.prefix.clone()
    }

    fn go_back(&mut self) {
        self.history.back();
    }

    fn go_forward(&mut self) {
        self.history.forward();
    }

    fn push(&mut self, path: String) {
        self.history.push(match &self.prefix {
            None => path,
            Some(prefix) => format!("{prefix}{path}"),
        });
    }

    fn replace(&mut self, path: String) {
        self.history.replace(match &self.prefix {
            None => path,
            Some(prefix) => format!("{prefix}{path}"),
        });
    }

    fn external(&mut self, url: String) -> bool {
        match self.window.location().set_href(&url) {
            Ok(_) => true,
            Err(e) => {
                error!("failed to navigate to external url (`{url}): {e:?}");
                false
            }
        }
    }

    fn updater(&mut self, callback: std::sync::Arc<dyn Fn() + Send + Sync>) {
        self.listener_navigation = Some(self.history.listen(move || (*callback)()));
    }
}

use gloo::{events::EventListener, utils::window};
use log::error;
use url::Url;
use wasm_bindgen::JsValue;
use web_sys::{History, Window};

use super::HistoryProvider;

const INITIAL_URL: &str = "dioxus-router-core://initial_url.invalid/";

/// A [`HistoryProvider`] that integrates with a browser via the [History API]. It uses the URLs
/// hash instead of its path.
///
/// Early web applications used the hash to store the current path because there was no other way
/// for them to interact with the history without triggering a browser navigation, as the
/// [History API] did not yet exist. While this implementation could have been written that way, it
/// was not, because no browser supports WebAssembly without the [History API].
///
/// [History API]: https://developer.mozilla.org/en-US/docs/Web/API/History_API
pub struct WebHashHistory {
    history: History,
    window: Window,
}

impl Default for WebHashHistory {
    fn default() -> Self {
        let window = window();

        Self {
            history: window.history().expect("window has history"),
            window,
        }
    }
}

impl WebHashHistory {
    fn join_url_to_hash(&self, path: String) -> Option<String> {
        if path.starts_with("//") {
            error!("cannot navigate to paths starting with `//`, got `{path}`");
            return None;
        }

        let url = match self.url() {
            Some(c) => match c.join(&path) {
                Ok(new) => new,
                Err(e) => {
                    error!("failed to join location with target: {e}");
                    return None;
                }
            },
            None => {
                error!("current location unknown");
                return None;
            }
        };

        Some(format!(
            "#{path}{query}",
            path = url.path(),
            query = url.query().map(|q| format!("?{q}")).unwrap_or_default()
        ))
    }

    fn url(&self) -> Option<Url> {
        let mut path = self.window.location().hash().ok()?;

        if path.starts_with('#') {
            path.remove(0);
        }

        if path.starts_with('/') {
            path.remove(0);
        }

        match Url::parse(&format!("{INITIAL_URL}/{path}")) {
            Ok(url) => Some(url),
            Err(e) => {
                error!("failed to parse hash path: {e}");
                None
            }
        }
    }
}

impl HistoryProvider for WebHashHistory {
    fn current_path(&self) -> String {
        self.url()
            .map(|url| url.path().to_string())
            .unwrap_or(String::from("/"))
    }

    fn current_query(&self) -> Option<String> {
        self.url().and_then(|url| url.query().map(String::from))
    }

    fn current_prefix(&self) -> Option<String> {
        Some(String::from("#"))
    }

    fn go_back(&mut self) {
        if let Err(e) = self.history.back() {
            error!("failed to go back: {e:?}")
        }
    }

    fn go_forward(&mut self) {
        if let Err(e) = self.history.forward() {
            error!("failed to go forward: {e:?}")
        }
    }

    fn push(&mut self, path: String) {
        let hash = match self.join_url_to_hash(path) {
            Some(hash) => hash,
            None => return,
        };

        if let Err(e) = self
            .history
            .push_state_with_url(&JsValue::NULL, "", Some(&hash))
        {
            error!("failed to push state: {e:?}");
        }
    }

    fn replace(&mut self, path: String) {
        let hash = match self.join_url_to_hash(path) {
            Some(hash) => hash,
            None => return,
        };

        if let Err(e) = self
            .history
            .replace_state_with_url(&JsValue::NULL, "", Some(&hash))
        {
            error!("failed to replace state: {e:?}");
        }
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
        let listener = EventListener::new(&self.window, "popstate", move |_| (*callback)());
    }
}

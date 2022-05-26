use std::sync::Arc;

use gloo_events::EventListener;
use log::error;
use url::Url;
use wasm_bindgen::JsValue;
use web_sys::{History, ScrollRestoration, Window};

use super::{update_history_with_scroll, HistoryProvider, ScrollPosition};

/// A [`HistoryProvider`] that stores the current path and query in the browsers URL fragment.
///
/// Uses the [History API] to integrate with the browsers history. Also stores the current scroll
/// position and restores it when traversing the history.
///
/// [History API]: https://developer.mozilla.org/en-US/docs/Web/API/History_API
pub struct BrowserHashHistoryProvider {
    history: History,
    listener_navigation: Option<EventListener>,
    _listener_scroll: EventListener,
    window: Window,
}

impl BrowserHashHistoryProvider {
    /// Get the current url from the hash.
    fn url(&self) -> Option<Url> {
        let mut path = self.window.location().hash().ok()?;

        if path.starts_with('#') {
            path.remove(0);
        }

        if !path.starts_with('/') {
            path = format!("/{path}");
        }

        match Url::parse(&format!("dioxus://index.html{path}")) {
            Ok(url) => Some(url),
            Err(e) => {
                error!("failed to parse hash path: {e:?}");
                None
            }
        }
    }
}

impl Default for BrowserHashHistoryProvider {
    fn default() -> Self {
        let window = web_sys::window().unwrap();
        let history = window.history().unwrap();

        // disable browser scroll behaviour
        if let Err(e) = history.set_scroll_restoration(ScrollRestoration::Manual) {
            error!("failed to change to manual scroll restoration: {e:?}")
        }

        let listener_scroll = {
            let inner_window = window.clone();
            let history = history.clone();
            EventListener::new(
                &window.document().expect("access to document"),
                "scroll",
                move |_| update_history_with_scroll(&inner_window, &history),
            )
        };

        Self {
            history,
            listener_navigation: None,
            _listener_scroll: listener_scroll,
            window,
        }
    }
}

impl HistoryProvider for BrowserHashHistoryProvider {
    fn foreign_navigation_handler(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
        let history = self.history.clone();
        let window = self.window.clone();

        // replace listener
        self.listener_navigation = Some(EventListener::new(&self.window, "popstate", move |_| {
            // tell router to update
            callback();

            // update scroll position
            let ScrollPosition { x, y } = history
                .state()
                .map(|state| state.into_serde().unwrap_or_default())
                .unwrap_or_default();
            // TODO: find way to scroll when new outlets are updated
            window.scroll_to_with_x_and_y(x, y);
        }));
    }

    fn current_path(&self) -> String {
        match self.url() {
            Some(url) => url.path().to_string(),
            None => String::from("/"),
        }
    }

    fn current_prefix(&self) -> String {
        String::from("#")
    }

    fn current_query(&self) -> Option<String> {
        self.url()
            .map(|url| url.query().map(|query| query.to_string()))
            .flatten()
    }

    fn go_back(&mut self) {
        if let Err(e) = self.history.back() {
            error!("failed to navigate back: {e:?}");
        }
    }

    fn go_forward(&mut self) {
        if let Err(e) = self.history.forward() {
            error!("failed to navigate forward: {e:?}");
        }
    }

    fn push(&mut self, path: String) {
        if path.starts_with("//") {
            error!(r#"cannot navigate to paths starting with "//", path: {path}"#);
            return;
        }

        // join path & get hash
        let hash = match self.url().map(|url| url.join(&path)) {
            Some(Ok(url)) => format!(
                "#{path}{query}",
                path = url.path(),
                query = url.query().map(|q| format!("?{q}")).unwrap_or_default()
            ),
            Some(Err(e)) => {
                error!("failed to join locations: {e}");
                return;
            }
            None => return,
        };

        match self.history.push_state_with_url(
            &JsValue::from_serde(&ScrollPosition::default()).unwrap(),
            "",
            Some(&hash),
        ) {
            Ok(_) => self.window.scroll_with_x_and_y(0.0, 0.0),
            Err(e) => error!("failed to push state: {e:?}"),
        }
    }

    fn replace(&mut self, path: String) {
        if path.starts_with("//") {
            error!(r#"cannot navigate to paths starting with "//", path: {path}"#);
            return;
        }

        // join path & get hash
        let hash = match self.url().map(|url| url.join(&path)) {
            Some(Ok(url)) => format!(
                "#{path}{query}",
                path = url.path(),
                query = url.query().map(|q| format!("?{q}")).unwrap_or_default()
            ),
            Some(Err(e)) => {
                error!("failed to join locations: {e}");
                return;
            }
            None => return,
        };

        match self.history.replace_state_with_url(
            &JsValue::from_serde(&ScrollPosition::default()).unwrap(),
            "",
            Some(&hash),
        ) {
            Ok(_) => self.window.scroll_with_x_and_y(0.0, 0.0),
            Err(e) => error!("failed to push state: {e:?}"),
        };
    }

    fn can_external(&self) -> bool {
        true
    }

    fn external(&self, url: String) {
        self.window.location().set_href(&url).ok();
    }
}

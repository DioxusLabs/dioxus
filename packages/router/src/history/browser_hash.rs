use std::sync::Arc;

use gloo_events::EventListener;
use url::Url;
use wasm_bindgen::JsValue;
use web_sys::{History, Window};

use super::HistoryProvider;

/// A [`HistoryProvider`] that uses the [History API] and [Location API] to integrate with the
/// browser.
///
/// [History API]: https://developer.mozilla.org/en-US/docs/Web/API/History_API
/// [Location API]: https://developer.mozilla.org/en-US/docs/Web/API/Location
pub struct BrowserHashHistoryProvider {
    history: History,
    listener: Option<EventListener>,
    window: Window,
}

impl Default for BrowserHashHistoryProvider {
    fn default() -> Self {
        let window = web_sys::window().unwrap();
        let history = window.history().unwrap();

        Self {
            history,
            listener: Default::default(),
            window,
        }
    }
}

impl HistoryProvider for BrowserHashHistoryProvider {
    fn foreign_navigation_handler(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
        // recreate event listener
        self.listener = Some(EventListener::new(&self.window, "popstate", move |_| {
            callback()
        }));
    }

    fn current_path(&self) -> String {
        let mut p = self
            .window
            .location()
            .hash()
            .expect("location can provide hash");

        if p.starts_with('#') {
            p.remove(0);
        }

        if !p.starts_with('/') {
            p = format!("/{p}");
        }

        if let Some(url) = Url::parse(&format!("dioxus://index.html{p}")).ok() {
            url.path().to_string()
        } else {
            String::from("/")
        }
    }

    fn current_prefix(&self) -> String {
        String::from("#")
    }

    fn current_query(&self) -> Option<String> {
        let mut p = self
            .window
            .location()
            .hash()
            .expect("location can provide hash");

        if p.starts_with('#') {
            p.remove(0);
        }

        if !p.starts_with('/') {
            p = format!("/{p}");
        }

        if let Some(url) = Url::parse(&format!("dioxus://index.html{p}")).ok() {
            url.query().map(|q| q.to_string())
        } else {
            None
        }
    }

    fn go_back(&mut self) {
        self.history.back().ok();
    }

    fn go_forward(&mut self) {
        self.history.forward().ok();
    }

    fn push(&mut self, path: String) {
        let mut p = self.window.location().hash().unwrap_or_default();

        if p.starts_with('#') {
            p.remove(0);
        }

        if !p.starts_with('/') {
            p = format!("/{p}");
        }

        let hash = match Url::parse(&format!("dioxus://index.html{p}")).map(|url| url.join(&path)) {
            Ok(Ok(url)) => {
                let path = url.path();
                let query = url.query().map(|q| format!("?{q}")).unwrap_or_default();
                format!("#{path}{query}")
            }
            _ => return,
        };

        self.history
            .push_state_with_url(&JsValue::NULL, "", Some(&hash))
            .ok();
    }

    fn replace(&mut self, path: String) {
        let mut p = self
            .window
            .location()
            .hash()
            .expect("location can provide hash");

        if p.starts_with('#') {
            p.remove(0);
        }

        if !p.starts_with('/') {
            p = format!("/{p}");
        }

        let hash = match Url::parse(&format!("dioxus://index.html{p}")).map(|url| url.join(&path)) {
            Ok(Ok(url)) => {
                let path = url.path();
                let query = url.query().map(|q| format!("?{q}")).unwrap_or_default();
                format!("#{path}{query}")
            }
            _ => return,
        };

        self.history
            .replace_state_with_url(&JsValue::NULL, "", Some(&hash))
            .ok();
    }
}

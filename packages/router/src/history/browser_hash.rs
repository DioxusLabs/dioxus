use std::sync::Arc;

use gloo_events::EventListener;
use url::Url;
use wasm_bindgen::JsValue;
use web_sys::{History, HtmlElement, Window};

use super::{HistoryProvider, ScrollPosition};

/// A [`HistoryProvider`] that stores the current path and query in the browsers URL fragment.
///
/// Uses the [History API] to integrate with the browsers history. Also stores the current scroll
/// position and restores it when traversing the history.
///
/// [History API]: https://developer.mozilla.org/en-US/docs/Web/API/History_API
pub struct BrowserHashHistoryProvider {
    body: HtmlElement,
    history: History,
    listener: Option<EventListener>,
    window: Window,
}

impl Default for BrowserHashHistoryProvider {
    fn default() -> Self {
        let window = web_sys::window().unwrap();
        let body = window.document().unwrap().body().unwrap();
        let history = window.history().unwrap();

        Self {
            body,
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

        let ScrollPosition { x, y } = self.history.state().unwrap().into_serde().unwrap();
        self.body.set_scroll_top(y);
        self.body.set_scroll_left(x);
    }

    fn go_forward(&mut self) {
        self.history.forward().ok();

        let ScrollPosition { x, y } = self.history.state().unwrap().into_serde().unwrap();
        self.body.set_scroll_top(y);
        self.body.set_scroll_left(x);
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

        if let Ok(_) = self.history.push_state_with_url(
            &JsValue::from_serde(&ScrollPosition {
                x: self.body.scroll_left(),
                y: self.body.scroll_top(),
            })
            .unwrap(),
            "",
            Some(&hash),
        ) {
            self.body.set_scroll_top(0);
            self.body.set_scroll_left(0);
        }
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

        if let Ok(_) = self.history.replace_state_with_url(
            &JsValue::from_serde(&ScrollPosition {
                x: self.body.scroll_left(),
                y: self.body.scroll_top(),
            })
            .unwrap(),
            "",
            Some(&hash),
        ) {
            self.body.set_scroll_top(0);
            self.body.set_scroll_left(0);
        };
    }

    fn can_external(&self) -> bool {
        true
    }

    fn external(&self, url: String) {
        self.window.location().set_href(&url).ok();
    }
}

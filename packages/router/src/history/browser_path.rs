use std::sync::Arc;

use gloo_events::EventListener;
use log::error;
use wasm_bindgen::JsValue;
use web_sys::{History, ScrollRestoration, Window};

use super::{update_history_with_scroll, HistoryProvider, ScrollPosition};

/// A [`HistoryProvider`] that uses the [History API] and [Location] to integrate with the
/// browser.
///
/// # Prefix
/// This [`HistoryProvider`] supports a prefix, which allows its use for web apps not located at the
/// root of their domain.
///
/// It is up to the application developer to ensure the prefix ends at a `/`. Otherwise, the first
/// navigation from within the app will add one.
///
/// Application developers are responsible for unmounting the router or app when the prefix isn't
/// present in the current URL. If the router is rendered and a navigation is caused, the prefix
/// will be introduced to the URL.
///
/// # Example
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// use dioxus::router::history::BrowserPathHistoryProvider;
/// fn App(cx: Scope) -> Element {
///     let routes = use_segment(&cx, Segment::default);
///
///     cx.render(rsx! {
///         Router {
///             routes: routes.clone(),
///             history: &|| BrowserPathHistoryProvider::with_prefix(String::from("/pre")),
///             Outlet { }
///         }
///     })
/// }
/// ```
///
/// [History API]: https://developer.mozilla.org/en-US/docs/Web/API/History_API
/// [Location]: https://developer.mozilla.org/en-US/docs/Web/API/Location
pub struct WebHistory {
    history: History,
    listener_navigation: Option<EventListener>,
    _listener_scroll: EventListener,
    prefix: Option<String>,
    window: Window,
}

impl WebHistory {
    /// Create a new [`WebHistory`] with a prefix.
    #[must_use]
    pub fn with_prefix(prefix: String) -> Box<Self> {
        Box::new(Self {
            prefix: Some(prefix),
            ..Default::default()
        })
    }
}

impl Default for WebHistory {
    fn default() -> Self {
        let window = web_sys::window().expect("access to window");
        let history = window.history().expect("access to history");

        if let Err(e) = history.set_scroll_restoration(ScrollRestoration::Manual) {
            error!("failed to change to manual scroll restoration: {e:?}");
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
            prefix: None,
            window,
        }
    }
}

impl HistoryProvider for WebHistory {
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
        let mut path = self.window.location().pathname().unwrap_or_default();

        // strip prefix if present
        if let Some(prefix) = &self.prefix {
            if path.starts_with(prefix) {
                path = path.split_at(prefix.len()).1.to_string();
            }
        }

        // ensure path starts with /
        if !path.starts_with('/') {
            path = format!("/{path}");
        }

        path
    }

    fn current_prefix(&self) -> String {
        self.prefix.clone().unwrap_or_default()
    }

    fn current_query(&self) -> Option<String> {
        let mut query = self.window.location().search().ok()?;

        // remove ? from start of query
        if query.starts_with('?') {
            query.remove(0);
        }

        match query.is_empty() {
            false => Some(query),
            true => None,
        }
    }

    fn go_back(&mut self) {
        if let Err(e) = self.history.back() {
            error!("failed to navigate back: {e:?}");
        }
    }

    fn go_forward(&mut self) {
        if let Err(e) = self.history.forward() {
            error!("failed to navigate forward: {e:?}")
        }
    }

    fn push(&mut self, mut path: String) {
        if path.starts_with("//") {
            error!(r#"cannot navigate to paths starting with "//", path: {path}"#);
            return;
        }

        if let (Some(prefix), true) = (&self.prefix, path.starts_with('/')) {
            path = format!("{prefix}{path}");
        }

        match self.history.push_state_with_url(
            &JsValue::from_serde(&ScrollPosition::default()).unwrap(),
            "",
            Some(&path),
        ) {
            Ok(_) => self.window.scroll_to_with_x_and_y(0.0, 0.0),
            Err(e) => error!("failed to push state: {e:?}"),
        }
    }

    fn replace(&mut self, mut path: String) {
        if path.starts_with("//") {
            error!(r#"cannot navigate to paths starting with "//", path: {path}"#);
            return;
        }

        if let (Some(prefix), true) = (&self.prefix, path.starts_with('/')) {
            path = format!("{prefix}{path}");
        }

        match self.history.replace_state_with_url(
            &JsValue::from_serde(&ScrollPosition::default()).unwrap(),
            "",
            Some(&path),
        ) {
            Ok(_) => self.window.scroll_to_with_x_and_y(0.0, 0.0),
            Err(e) => error!("failed to replace state: {e:?}"),
        }
    }

    fn can_external(&self) -> bool {
        true
    }

    fn external(&self, url: String) {
        if let Err(e) = self.window.location().set_href(&url) {
            error!("failed to navigate to external href: {e:?}");
        }
    }
}

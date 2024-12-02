use scroll::ScrollPosition;
use wasm_bindgen::JsCast;
use wasm_bindgen::{prelude::Closure, JsValue};
use web_sys::{window, Window};
use web_sys::{Event, History, ScrollRestoration};

mod scroll;

fn base_path() -> Option<String> {
    let base_path = dioxus_cli_config::web_base_path();
    tracing::trace!("Using base_path from the CLI: {:?}", base_path);
    base_path
}

#[allow(clippy::extra_unused_type_parameters)]
fn update_scroll(window: &Window, history: &History) {
    let scroll = ScrollPosition::of_window(window);
    if let Err(err) = replace_state_with_url(history, &[scroll.x, scroll.y], None) {
        web_sys::console::error_1(&err);
    }
}

/// A [`HistoryProvider`] that integrates with a browser via the [History API](https://developer.mozilla.org/en-US/docs/Web/API/History_API).
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
pub struct WebHistory {
    do_scroll_restoration: bool,
    history: History,
    prefix: Option<String>,
    window: Window,
}

impl Default for WebHistory {
    fn default() -> Self {
        Self::new(None, true)
    }
}

impl WebHistory {
    /// Create a new [`WebHistory`].
    ///
    /// If `do_scroll_restoration` is [`true`], [`WebHistory`] will take control of the history
    /// state. It'll also set the browsers scroll restoration to `manual`.
    pub fn new(prefix: Option<String>, do_scroll_restoration: bool) -> Self {
        let myself = Self::new_inner(prefix, do_scroll_restoration);

        let current_route = dioxus_history::History::current_route(&myself);
        let current_route_str = current_route.to_string();
        let prefix_str = myself.prefix.as_deref().unwrap_or("");
        let current_url = format!("{prefix_str}{current_route_str}");
        let state = myself.create_state();
        let _ = replace_state_with_url(&myself.history, &state, Some(&current_url));

        myself
    }

    fn new_inner(prefix: Option<String>, do_scroll_restoration: bool) -> Self {
        let window = window().expect("access to `window`");
        let history = window.history().expect("`window` has access to `history`");

        if do_scroll_restoration {
            history
                .set_scroll_restoration(ScrollRestoration::Manual)
                .expect("`history` can set scroll restoration");
        }

        let prefix = prefix
            // If there isn't a base path, try to grab one from the CLI
            .or_else(base_path)
            // Normalize the prefix to start and end with no slashes
            .as_ref()
            .map(|prefix| prefix.trim_matches('/'))
            // If the prefix is empty, don't add it
            .filter(|prefix| !prefix.is_empty())
            // Otherwise, start with a slash
            .map(|prefix| format!("/{prefix}"));

        Self {
            do_scroll_restoration,
            history,
            prefix,
            window,
        }
    }

    fn scroll_pos(&self) -> ScrollPosition {
        self.do_scroll_restoration
            .then(|| ScrollPosition::of_window(&self.window))
            .unwrap_or_default()
    }

    fn create_state(&self) -> [f64; 2] {
        let scroll = self.scroll_pos();
        [scroll.x, scroll.y]
    }
}

impl WebHistory {
    fn route_from_location(&self) -> String {
        let location = self.window.location();
        let path = location.pathname().unwrap_or_else(|_| "/".into())
            + &location.search().unwrap_or("".into())
            + &location.hash().unwrap_or("".into());
        let mut path = match self.prefix {
            None => &path,
            Some(ref prefix) => path.strip_prefix(prefix).unwrap_or(prefix),
        };
        // If the path is empty, parse the root route instead
        if path.is_empty() {
            path = "/"
        }
        path.to_string()
    }

    fn full_path(&self, state: &String) -> String {
        match &self.prefix {
            None => state.to_string(),
            Some(prefix) => format!("{prefix}{state}"),
        }
    }

    fn handle_nav(&self, result: Result<(), JsValue>) {
        match result {
            Ok(_) => {
                if self.do_scroll_restoration {
                    self.window.scroll_to_with_x_and_y(0.0, 0.0)
                }
            }
            Err(e) => {
                web_sys::console::error_2(&JsValue::from_str("failed to change state: "), &e);
            }
        }
    }

    fn navigate_external(&self, url: String) -> bool {
        match self.window.location().set_href(&url) {
            Ok(_) => true,
            Err(e) => {
                web_sys::console::error_4(
                    &JsValue::from_str("failed to navigate to external url ("),
                    &JsValue::from_str(&url),
                    &JsValue::from_str("): "),
                    &e,
                );
                false
            }
        }
    }
}

impl dioxus_history::History for WebHistory {
    fn current_route(&self) -> String {
        self.route_from_location()
    }

    fn current_prefix(&self) -> Option<String> {
        self.prefix.clone()
    }

    fn go_back(&self) {
        if let Err(e) = self.history.back() {
            web_sys::console::error_2(&JsValue::from_str("failed to go back: "), &e);
        }
    }

    fn go_forward(&self) {
        if let Err(e) = self.history.forward() {
            web_sys::console::error_2(&JsValue::from_str("failed to go forward: "), &e);
        }
    }

    fn push(&self, state: String) {
        if state == self.current_route() {
            // don't push the same state twice
            return;
        }

        let w = window().expect("access to `window`");
        let h = w.history().expect("`window` has access to `history`");

        // update the scroll position before pushing the new state
        update_scroll(&w, &h);

        let path = self.full_path(&state);

        let state: [f64; 2] = self.create_state();
        self.handle_nav(push_state_and_url(&self.history, &state, path));
    }

    fn replace(&self, state: String) {
        let path = self.full_path(&state);

        let state = self.create_state();
        self.handle_nav(replace_state_with_url(&self.history, &state, Some(&path)));
    }

    fn external(&self, url: String) -> bool {
        self.navigate_external(url)
    }

    fn updater(&self, callback: std::sync::Arc<dyn Fn() + Send + Sync>) {
        let w = self.window.clone();
        let h = self.history.clone();
        let d = self.do_scroll_restoration;

        let function = Closure::wrap(Box::new(move |_| {
            (*callback)();
            if d {
                if let Some([x, y]) = get_current(&h) {
                    ScrollPosition { x, y }.scroll_to(w.clone())
                }
            }
        }) as Box<dyn FnMut(Event)>);
        self.window
            .add_event_listener_with_callback(
                "popstate",
                &function.into_js_value().unchecked_into(),
            )
            .unwrap();
    }
}

pub(crate) fn replace_state_with_url(
    history: &History,
    value: &[f64; 2],
    url: Option<&str>,
) -> Result<(), JsValue> {
    let position = js_sys::Array::new();
    position.push(&JsValue::from(value[0]));
    position.push(&JsValue::from(value[1]));

    history.replace_state_with_url(&position, "", url)
}

pub(crate) fn push_state_and_url(
    history: &History,
    value: &[f64; 2],
    url: String,
) -> Result<(), JsValue> {
    let position = js_sys::Array::new();
    position.push(&JsValue::from(value[0]));
    position.push(&JsValue::from(value[1]));

    history.push_state_with_url(&position, "", Some(&url))
}

pub(crate) fn get_current(history: &History) -> Option<[f64; 2]> {
    use wasm_bindgen::JsCast;

    let state = history.state();
    if let Err(err) = &state {
        web_sys::console::error_1(err);
    }
    state.ok().and_then(|state| {
        let state = state.dyn_into::<js_sys::Array>().ok()?;
        let x = state.get(0).as_f64()?;
        let y = state.get(1).as_f64()?;
        Some([x, y])
    })
}

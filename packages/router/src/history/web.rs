use std::sync::{Arc, Mutex};

use gloo::{console::error, events::EventListener, render::AnimationFrame};

use wasm_bindgen::JsValue;
use web_sys::{window, History, ScrollRestoration, Window};

use crate::routable::Routable;

use super::{
    web_history::{get_current, push_state_and_url, replace_state_with_url},
    web_scroll::ScrollPosition,
    HistoryProvider,
};

#[allow(dead_code)]
fn base_path() -> Option<&'static str> {
    todo!("set basepath not through compile-time env vars!")
    // tracing::trace!(
    //     "Using base_path from Dioxus.toml: {:?}",
    //     dioxus_cli_config::base_path()
    // );
    // dioxus_cli_config::base_path()
}

#[allow(clippy::extra_unused_type_parameters)]
fn update_scroll<R>(window: &Window, history: &History) {
    let scroll = ScrollPosition::of_window(window);
    if let Err(err) = replace_state_with_url(history, &[scroll.x, scroll.y], None) {
        error!(err);
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
pub struct WebHistory<R: Routable> {
    do_scroll_restoration: bool,
    history: History,
    listener_navigation: Option<EventListener>,
    listener_animation_frame: Arc<Mutex<Option<AnimationFrame>>>,
    prefix: Option<String>,
    window: Window,
    phantom: std::marker::PhantomData<R>,
}

impl<R: Routable> Default for WebHistory<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self::new(None, true)
    }
}

impl<R: Routable> WebHistory<R> {
    /// Create a new [`WebHistory`].
    ///
    /// If `do_scroll_restoration` is [`true`], [`WebHistory`] will take control of the history
    /// state. It'll also set the browsers scroll restoration to `manual`.
    pub fn new(prefix: Option<String>, do_scroll_restoration: bool) -> Self
    where
        <R as std::str::FromStr>::Err: std::fmt::Display,
    {
        let myself = Self::new_inner(prefix, do_scroll_restoration);

        let current_route = myself.current_route();
        let current_route_str = current_route.to_string();
        let prefix_str = myself.prefix.as_deref().unwrap_or("");
        let current_url = format!("{prefix_str}{current_route_str}");
        let state = myself.create_state(current_route);
        let _ = replace_state_with_url(&myself.history, &state, Some(&current_url));

        myself
    }

    fn new_inner(prefix: Option<String>, do_scroll_restoration: bool) -> Self
    where
        <R as std::str::FromStr>::Err: std::fmt::Display,
    {
        let window = window().expect("access to `window`");
        let history = window.history().expect("`window` has access to `history`");

        if do_scroll_restoration {
            history
                .set_scroll_restoration(ScrollRestoration::Manual)
                .expect("`history` can set scroll restoration");
        }

        let prefix = prefix
            // If there isn't a base path, try to grab one from the CLI
            .or_else(|| base_path().map(|s| s.to_string()))
            // Normalize the prefix to start and end with no slashes
            .map(|prefix| prefix.trim_matches('/').to_string())
            // If the prefix is empty, don't add it
            .filter(|prefix| !prefix.is_empty())
            // Otherwise, start with a slash
            .map(|prefix| format!("/{prefix}"));

        Self {
            do_scroll_restoration,
            history,
            listener_navigation: None,
            listener_animation_frame: Default::default(),
            prefix,
            window,
            phantom: Default::default(),
        }
    }

    fn scroll_pos(&self) -> ScrollPosition {
        self.do_scroll_restoration
            .then(|| ScrollPosition::of_window(&self.window))
            .unwrap_or_default()
    }

    fn create_state(&self, _state: R) -> [f64; 2] {
        let scroll = self.scroll_pos();
        [scroll.x, scroll.y]
    }
}

impl<R: Routable> WebHistory<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn route_from_location(&self) -> R {
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
        R::from_str(path).unwrap_or_else(|err| panic!("{}", err))
    }

    fn full_path(&self, state: &R) -> String {
        match &self.prefix {
            None => format!("{state}"),
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
            Err(e) => error!("failed to change state: ", e),
        }
    }

    fn navigate_external(&mut self, url: String) -> bool {
        match self.window.location().set_href(&url) {
            Ok(_) => true,
            Err(e) => {
                error!("failed to navigate to external url (", url, "): ", e);
                false
            }
        }
    }
}

impl<R: Routable> HistoryProvider<R> for WebHistory<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn current_route(&self) -> R {
        self.route_from_location()
    }

    fn current_prefix(&self) -> Option<String> {
        self.prefix.clone()
    }

    fn go_back(&mut self) {
        if let Err(e) = self.history.back() {
            error!("failed to go back: ", e)
        }
    }

    fn go_forward(&mut self) {
        if let Err(e) = self.history.forward() {
            error!("failed to go forward: ", e)
        }
    }

    fn push(&mut self, state: R) {
        if state.to_string() == self.current_route().to_string() {
            // don't push the same state twice
            return;
        }

        let w = window().expect("access to `window`");
        let h = w.history().expect("`window` has access to `history`");

        // update the scroll position before pushing the new state
        update_scroll::<R>(&w, &h);

        let path = self.full_path(&state);

        let state: [f64; 2] = self.create_state(state);
        self.handle_nav(push_state_and_url(&self.history, &state, path));
    }

    fn replace(&mut self, state: R) {
        let path = self.full_path(&state);

        let state = self.create_state(state);
        self.handle_nav(replace_state_with_url(&self.history, &state, Some(&path)));
    }

    fn external(&mut self, url: String) -> bool {
        self.navigate_external(url)
    }

    fn updater(&mut self, callback: std::sync::Arc<dyn Fn() + Send + Sync>) {
        let w = self.window.clone();
        let h = self.history.clone();
        let s = self.listener_animation_frame.clone();
        let d = self.do_scroll_restoration;

        self.listener_navigation = Some(EventListener::new(&self.window, "popstate", move |_| {
            (*callback)();
            if d {
                let mut s = s.lock().expect("unpoisoned scroll mutex");
                if let Some([x, y]) = get_current(&h) {
                    *s = Some(ScrollPosition { x, y }.scroll_to(w.clone()));
                }
            }
        }));
    }
}

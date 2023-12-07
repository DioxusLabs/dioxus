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

#[cfg(not(feature = "serde"))]
#[allow(clippy::extra_unused_type_parameters)]
fn update_scroll<R>(window: &Window, history: &History) {
    let scroll = ScrollPosition::of_window(window);
    if let Err(err) = replace_state_with_url(history, &[scroll.x, scroll.y], None) {
        error!(err);
    }
}

#[cfg(feature = "serde")]
fn update_scroll<R: serde::Serialize + serde::de::DeserializeOwned + Routable>(
    window: &Window,
    history: &History,
) {
    if let Some(WebHistoryState { state, .. }) = get_current::<WebHistoryState<R>>(history) {
        let scroll = ScrollPosition::of_window(window);
        let state = WebHistoryState { state, scroll };
        if let Err(err) = replace_state_with_url(history, &state, None) {
            error!(err);
        }
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize, serde::Serialize)]
struct WebHistoryState<R> {
    state: R,
    scroll: ScrollPosition,
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

#[cfg(not(feature = "serde"))]
impl<R: Routable> Default for WebHistory<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self::new(None, true)
    }
}

#[cfg(feature = "serde")]
impl<R: Routable> Default for WebHistory<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
    R: serde::Serialize + serde::de::DeserializeOwned,
{
    fn default() -> Self {
        Self::new(None, true)
    }
}

impl<R: Routable> WebHistory<R> {
    #[cfg(not(feature = "serde"))]
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
        let current_url = current_route.to_string();
        let state = myself.create_state(current_route);
        let _ = replace_state_with_url(&myself.history, &state, Some(&current_url));

        myself
    }

    #[cfg(feature = "serde")]
    /// Create a new [`WebHistory`].
    ///
    /// If `do_scroll_restoration` is [`true`], [`WebHistory`] will take control of the history
    /// state. It'll also set the browsers scroll restoration to `manual`.
    pub fn new(prefix: Option<String>, do_scroll_restoration: bool) -> Self
    where
        <R as std::str::FromStr>::Err: std::fmt::Display,
        R: serde::Serialize + serde::de::DeserializeOwned,
    {
        let w = window().expect("access to `window`");
        let h = w.history().expect("`window` has access to `history`");
        let document = w.document().expect("`window` has access to `document`");

        let myself = Self::new_inner(
            prefix,
            do_scroll_restoration,
            EventListener::new(&document, "scroll", {
                let mut last_updated = 0.0;
                move |evt| {
                    // the time stamp in milliseconds
                    let time_stamp = evt.time_stamp();
                    // throttle the scroll event to 100ms
                    if (time_stamp - last_updated) < 100.0 {
                        return;
                    }
                    update_scroll::<R>(&w, &h);
                    last_updated = time_stamp;
                }
            }),
        );

        let current_route = myself.current_route();
        tracing::trace!("initial route: {:?}", current_route);
        let current_url = current_route.to_string();
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

    #[cfg(not(feature = "serde"))]
    fn create_state(&self, _state: R) -> [f64; 2] {
        let scroll = self.scroll_pos();
        [scroll.x, scroll.y]
    }

    #[cfg(feature = "serde")]
    fn create_state(&self, state: R) -> WebHistoryState<R> {
        let scroll = self.scroll_pos();
        WebHistoryState { state, scroll }
    }
}

impl<R: Routable> WebHistory<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn route_from_location(&self) -> R {
        let location = self.window.location();
        let path = location.pathname().unwrap_or_else(|_| "/".into())
            + &location.search().unwrap_or("".into());
        R::from_str(&path).unwrap_or_else(|err| panic!("{}", err))
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

#[cfg(feature = "serde")]
impl<R: serde::Serialize + serde::de::DeserializeOwned + Routable> HistoryProvider<R>
    for WebHistory<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn current_route(&self) -> R {
        match get_current::<WebHistoryState<_>>(&self.history) {
            // Try to get the route from the history state
            Some(route) => route.state,
            // If that fails, get the route from the current URL
            None => self.route_from_location(),
        }
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
        use gloo_utils::format::JsValueSerdeExt;
        if JsValue::from_serde(&state) != JsValue::from_serde(&self.current_route()) {
            // don't push the same state twice
            return;
        }

        let w = window().expect("access to `window`");
        let h = w.history().expect("`window` has access to `history`");

        // update the scroll position before pushing the new state
        update_scroll::<R>(&w, &h);

        let path = self.full_path(&state);

        let state = self.create_state(state);

        self.handle_nav(push_state_and_url(&self.history, &state, path));
    }

    fn replace(&mut self, state: R) {
        let path = match &self.prefix {
            None => format!("{state}"),
            Some(prefix) => format!("{prefix}{state}"),
        };

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
                if let Some(current_state) = get_current::<WebHistoryState<R>>(&h) {
                    *s = Some(current_state.scroll.scroll_to(w.clone()));
                }
            }
        }));
    }
}

#[cfg(not(feature = "serde"))]
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
        let path = match &self.prefix {
            None => format!("{state}"),
            Some(prefix) => format!("{prefix}{state}"),
        };

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

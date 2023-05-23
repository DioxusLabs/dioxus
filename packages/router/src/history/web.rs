use std::sync::{Arc, Mutex};

use gloo::{events::EventListener, render::AnimationFrame};
use log::error;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use web_sys::{window, History, ScrollRestoration, Window};

use crate::routable::Routable;

use super::{
    web_history::{get_current, push_state_and_url, replace_state_with_url},
    web_scroll::ScrollPosition,
    HistoryProvider,
};

#[derive(Deserialize, Serialize)]
struct WebHistoryState<R> {
    state: R,
    scroll: ScrollPosition,
}

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
pub struct WebHistory<R: Serialize + DeserializeOwned> {
    do_scroll_restoration: bool,
    history: History,
    listener_navigation: Option<EventListener>,
    #[allow(dead_code)]
    listener_scroll: Option<EventListener>,
    listener_animation_frame: Arc<Mutex<Option<AnimationFrame>>>,
    prefix: Option<String>,
    window: Window,
    phantom: std::marker::PhantomData<R>,
}

impl<R: Serialize + DeserializeOwned> WebHistory<R> {
    /// Create a new [`WebHistory`].
    ///
    /// If `do_scroll_restoration` is [`true`], [`WebHistory`] will take control of the history
    /// state. It'll also set the browsers scroll restoration to `manual`.
    pub fn new(prefix: Option<String>, do_scroll_restoration: bool) -> Self {
        let window = window().expect("access to `window`");
        let history = window.history().expect("`window` has access to `history`");

        let listener_scroll = match do_scroll_restoration {
            true => {
                history
                    .set_scroll_restoration(ScrollRestoration::Manual)
                    .expect("`history` can set scroll restoration");
                let w = window.clone();
                let h = history.clone();
                let document = w.document().expect("`window` has access to `document`");

                // Some(EventListener::new(&document, "scroll", move |_| {
                //     set_state(&w, &h);
                // }))
                None
            }
            false => None,
        };

        Self {
            do_scroll_restoration,
            history,
            listener_navigation: None,
            listener_scroll,
            listener_animation_frame: Default::default(),
            prefix,
            window,
            phantom: Default::default(),
        }
    }

    fn create_state(&self, state: R) -> WebHistoryState<R> {
        let scroll = self
            .do_scroll_restoration
            .then(|| ScrollPosition::of_window(&self.window))
            .unwrap_or_default();
        WebHistoryState { state, scroll }
    }
}

impl<R: Serialize + DeserializeOwned + Routable> HistoryProvider<R> for WebHistory<R>
where
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn current_route(&self) -> R {
        match get_current(&self.history) {
            // Try to get the route from the history state
            Some(route) => route,
            // If that fails, get the route from the current URL
            None => R::from_str(
                &self
                    .window
                    .location()
                    .pathname()
                    .unwrap_or_else(|_| String::from("/")),
            )
            .unwrap_or_else(|err| panic!("{}", err)),
        }
    }

    fn current_prefix(&self) -> Option<String> {
        self.prefix.clone()
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

    fn push(&mut self, state: R) {
        let path = match &self.prefix {
            None => format!("{state}"),
            Some(prefix) => format!("{prefix}{state}"),
        };

        let state = self.create_state(state);

        let nav = push_state_and_url(&self.history, &state, path);

        match nav {
            Ok(_) => {
                if self.do_scroll_restoration {
                    self.window.scroll_to_with_x_and_y(0.0, 0.0)
                }
            }
            Err(e) => error!("failed to push state: {e:?}"),
        }
    }

    fn replace(&mut self, state: R) {
        let path = match &self.prefix {
            None => format!("{state}"),
            Some(prefix) => format!("{prefix}{state}"),
        };

        let state = self.create_state(state);

        let nav = replace_state_with_url(&self.history, &state, Some(&path));

        match nav {
            Ok(_) => {
                if self.do_scroll_restoration {
                    self.window.scroll_to_with_x_and_y(0.0, 0.0)
                }
            }
            Err(e) => error!("failed to replace state: {e:?}"),
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
        let w = self.window.clone();
        let h = self.history.clone();
        let s = self.listener_animation_frame.clone();
        let d = self.do_scroll_restoration;

        self.listener_navigation = Some(EventListener::new(&self.window, "popstate", move |_| {
            (*callback)();
            if d {
                // let mut s = s.lock().expect("unpoisoned scroll mutex");
                // *s = Some(update_scroll(&w, &h));
            }
        }));
    }
}

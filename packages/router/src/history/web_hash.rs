use std::sync::{Arc, Mutex};

use gloo::{events::EventListener, render::AnimationFrame, utils::window};
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;
use url::Url;
use web_sys::{History, ScrollRestoration, Window};

use crate::routable::Routable;

use super::HistoryProvider;

const INITIAL_URL: &str = "dioxus-router-core://initial_url.invalid/";

/// A [`HistoryProvider`] that integrates with a browser via the [History API]. It uses the URLs
/// hash instead of its path.
///
/// Early web applications used the hash to store the current path because there was no other way
/// for them to interact with the history without triggering a browser navigation, as the
/// [History API](https://developer.mozilla.org/en-US/docs/Web/API/History_API) did not yet exist. While this implementation could have been written that way, it
/// was not, because no browser supports WebAssembly without the [History API].
pub struct WebHashHistory<R: Serialize + DeserializeOwned> {
    do_scroll_restoration: bool,
    history: History,
    listener_navigation: Option<EventListener>,
    #[allow(dead_code)]
    listener_scroll: Option<EventListener>,
    listener_animation_frame: Arc<Mutex<Option<AnimationFrame>>>,
    window: Window,
    phantom: std::marker::PhantomData<R>,
}

impl<R: Serialize + DeserializeOwned> WebHashHistory<R> {
    /// Create a new [`WebHashHistory`].
    ///
    /// If `do_scroll_restoration` is [`true`], [`WebHashHistory`] will take control of the history
    /// state. It'll also set the browsers scroll restoration to `manual`.
    pub fn new(do_scroll_restoration: bool) -> Self {
        let window = window();
        let history = window.history().expect("`window` has access to `history`");

        history
            .set_scroll_restoration(ScrollRestoration::Manual)
            .expect("`history` can set scroll restoration");

        let listener_scroll = match do_scroll_restoration {
            true => {
                history
                    .set_scroll_restoration(ScrollRestoration::Manual)
                    .expect("`history` can set scroll restoration");
                let w = window.clone();
                let h = history.clone();
                let document = w.document().expect("`window` has access to `document`");

                Some(EventListener::new(&document, "scroll", move |_| {
                    update_history(&w, &h);
                }))
            }
            false => None,
        };

        Self {
            do_scroll_restoration,
            history,
            listener_navigation: None,
            listener_scroll,
            listener_animation_frame: Default::default(),
            window,
            phantom: Default::default(),
        }
    }
}

impl<R: Serialize + DeserializeOwned> WebHashHistory<R> {
    fn join_url_to_hash(&self, path: R) -> Option<String> {
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

impl<R: Serialize + DeserializeOwned + Routable> HistoryProvider<R> for WebHashHistory<R> {
    fn current_route(&self) -> R {
        self.url()
            .map(|url| url.path().to_string())
            .unwrap_or(String::from("/"))
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

    fn push(&mut self, path: R) {
        let hash = match self.join_url_to_hash(path) {
            Some(hash) => hash,
            None => return,
        };

        let state = match self.do_scroll_restoration {
            true => top_left(),
            false => self.history.state().unwrap_or_default(),
        };

        let nav = self.history.push_state_with_url(&state, "", Some(&hash));

        match nav {
            Ok(_) => {
                if self.do_scroll_restoration {
                    self.window.scroll_to_with_x_and_y(0.0, 0.0)
                }
            }
            Err(e) => error!("failed to push state: {e:?}"),
        }
    }

    fn replace(&mut self, path: R) {
        let hash = match self.join_url_to_hash(path) {
            Some(hash) => hash,
            None => return,
        };

        let state = match self.do_scroll_restoration {
            true => top_left(),
            false => self.history.state().unwrap_or_default(),
        };

        let nav = self.history.replace_state_with_url(&state, "", Some(&hash));

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
                let mut s = s.lock().expect("unpoisoned scroll mutex");
                *s = Some(update_scroll(&w, &h));
            }
        }));
    }
}

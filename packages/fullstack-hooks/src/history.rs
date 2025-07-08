//! A history provider for fullstack apps that is compatible with hydration.

use std::cell::OnceCell;

use dioxus_core::{prelude::queue_effect, schedule_update};
use dioxus_fullstack_protocol::is_hydrating;
use dioxus_history::History;

// If we are currently in a scope and this is the first run then queue a rerender
// for after hydration
fn match_hydration<O>(
    during_hydration: impl FnOnce() -> O,
    after_hydration: impl FnOnce() -> O,
) -> O {
    if is_hydrating() {
        let update = schedule_update();
        queue_effect(move || update());
        during_hydration()
    } else {
        after_hydration()
    }
}

/// A history provider for fullstack apps that is compatible with hydration.
#[derive(Clone)]
pub struct FullstackHistory<H> {
    initial_route: OnceCell<String>,
    #[cfg(feature = "server")]
    in_hydration_context: std::cell::Cell<bool>,
    history: H,
}

impl<H> FullstackHistory<H> {
    /// Create a new `FullstackHistory` with the given history.
    pub fn new(history: H) -> Self {
        Self {
            initial_route: OnceCell::new(),
            #[cfg(feature = "server")]
            in_hydration_context: std::cell::Cell::new(false),
            history,
        }
    }

    /// Create a new `FullstackHistory` with the given history and initial route.
    pub fn new_server(history: H) -> Self
    where
        H: History,
    {
        let initial_route = history.current_route();
        let history = Self::new(history);
        history.initial_route.set(initial_route).unwrap();
        history
    }

    /// Get the initial route of the history.
    fn initial_route(&self) -> String {
        let entry = dioxus_fullstack_protocol::serialize_context().create_entry();
        let route = self.initial_route.get_or_init(|| {
            entry
                .get()
                .expect("Failed to get initial route from hydration context")
        });
        #[cfg(feature = "server")]
        if !self.in_hydration_context.get() {
            entry.insert(route, std::panic::Location::caller());
            self.in_hydration_context.set(true);
        }
        route.clone()
    }
}

impl<H: History> History for FullstackHistory<H> {
    fn current_prefix(&self) -> Option<String> {
        self.history.current_prefix()
    }

    fn can_go_back(&self) -> bool {
        match_hydration(|| false, || self.history.can_go_back())
    }

    fn can_go_forward(&self) -> bool {
        match_hydration(|| false, || self.history.can_go_forward())
    }

    fn external(&self, url: String) -> bool {
        self.history.external(url)
    }

    fn updater(&self, callback: std::sync::Arc<dyn Fn() + Send + Sync>) {
        self.history.updater(callback)
    }

    fn include_prevent_default(&self) -> bool {
        self.history.include_prevent_default()
    }

    fn current_route(&self) -> String {
        match_hydration(|| self.initial_route(), || self.history.current_route())
    }

    fn go_back(&self) {
        self.history.go_back();
    }

    fn go_forward(&self) {
        self.history.go_forward();
    }

    fn push(&self, route: String) {
        self.history.push(route);
    }

    fn replace(&self, path: String) {
        self.history.replace(path);
    }
}

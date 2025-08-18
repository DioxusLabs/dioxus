//! A history provider for fullstack apps that is compatible with hydration.

use std::{cell::RefCell, rc::Rc};

use dioxus_core::{provide_context, queue_effect, schedule_update, try_consume_context};
use dioxus_fullstack_protocol::{is_hydrating, SerializeContextEntry};
use dioxus_history::{history, provide_history_context, History};

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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedRouteContext {
    route: String,
}

pub(crate) fn finalize_route() {
    // This may run in tests without the full hydration context set up, if it does, then just
    // return without modifying the context
    let Some(entry) = try_consume_context::<RouteEntry>() else {
        return;
    };
    let entry = entry
        .entry
        .borrow_mut()
        .take()
        .expect("Failed to get initial route from hydration context");
    if cfg!(feature = "server") {
        let history = history();
        let initial_route = history.current_route();
        entry.insert(&initial_route, std::panic::Location::caller());
        provide_context(ResolvedRouteContext {
            route: initial_route,
        });
    } else if cfg!(feature = "web") {
        let initial_route = entry
            .get()
            .expect("Failed to get initial route from hydration context");
        provide_context(ResolvedRouteContext {
            route: initial_route,
        });
    }
}

/// Provide the fullstack history context. This interacts with the hydration context so it must
/// be called in the same order on the client and server after the hydration context is created
pub fn provide_fullstack_history_context<H: History + 'static>(history: H) {
    let entry = dioxus_fullstack_protocol::serialize_context().create_entry();
    provide_context(RouteEntry {
        entry: Rc::new(RefCell::new(Some(entry.clone()))),
    });
    provide_history_context(Rc::new(FullstackHistory::new(history)));
}

#[derive(Clone)]
struct RouteEntry {
    entry: Rc<RefCell<Option<SerializeContextEntry<String>>>>,
}

/// A history provider for fullstack apps that is compatible with hydration.
#[derive(Clone)]
struct FullstackHistory<H> {
    history: H,
}

impl<H> FullstackHistory<H> {
    /// Create a new `FullstackHistory` with the given history.
    pub fn new(history: H) -> Self {
        Self { history }
    }

    /// Get the initial route of the history.
    fn initial_route(&self) -> String
    where
        H: History,
    {
        match try_consume_context::<ResolvedRouteContext>() {
            Some(context) => context.route,
            None => self.history.current_route(),
        }
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

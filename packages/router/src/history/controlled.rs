use std::sync::{Arc, Mutex};

use super::HistoryProvider;

struct ControlledHistoryCore {
    callback: Option<Arc<dyn Fn() + Send + Sync>>,
    changed: bool,
    external: Option<String>,
    history: Box<dyn HistoryProvider>,
}

/// A [`HistoryProvider`] that can be controlled by a [`HistoryController`].
///
/// This can be used to control a [`Router`] from outside the VDOM. For more information, look at
/// the `ssr` example.
///
/// [`Router`]: crate::components::Router
#[derive(Clone)]
pub struct ControlledHistoryProvider {
    core: Arc<Mutex<ControlledHistoryCore>>,
}

impl HistoryProvider for ControlledHistoryProvider {
    fn foreign_navigation_handler(&mut self, callback: Arc<dyn Fn() + Send + Sync>) {
        let mut core = self.core.lock().unwrap();
        core.callback = Some(callback.clone());
        core.history.foreign_navigation_handler(callback);
    }

    fn current_path(&self) -> String {
        self.core.lock().unwrap().history.current_path()
    }

    fn current_prefix(&self) -> String {
        self.core.lock().unwrap().history.current_prefix()
    }

    fn current_query(&self) -> Option<String> {
        self.core.lock().unwrap().history.current_query()
    }

    fn can_go_back(&self) -> bool {
        self.core.lock().unwrap().history.can_go_back()
    }

    fn can_go_forward(&self) -> bool {
        self.core.lock().unwrap().history.can_go_forward()
    }

    fn go_back(&mut self) {
        let mut core = self.core.lock().unwrap();

        core.changed = true;
        core.external = None;
        core.history.go_back();
    }

    fn go_forward(&mut self) {
        let mut core = self.core.lock().unwrap();

        core.changed = true;
        core.external = None;
        core.history.go_forward();
    }

    fn push(&mut self, path: String) {
        let mut core = self.core.lock().unwrap();

        core.changed = true;
        core.external = None;
        core.history.push(path);
    }

    fn replace(&mut self, path: String) {
        let mut core = self.core.lock().unwrap();

        core.changed = true;
        core.external = None;
        core.history.replace(path);
    }

    fn can_external(&self) -> bool {
        true
    }

    fn external(&self, url: String) {
        let mut core = self.core.lock().unwrap();

        core.changed = true;
        core.external = Some(url.clone());
        if core.history.can_external() {
            core.history.external(url);
        }
    }
}

/// A [`HistoryProvider`] that controls a [`ControlledHistoryProvider`].
///
/// This can be used to control a [`Router`] from outside the VDOM. For more information, look at
/// the `ssr` example.
///
/// The [`HistoryController`] also implements [`HistoryProvider`] and causes the router to update
/// when the path is changed.
///
/// The [`HistoryController::has_redirected`] method can be used to check if the [`Router`] has
/// changed the current path/query. This is always reset when triggering a navigation from the
/// outside.
///
/// The [`HistoryController::get_external`] can be used to get the external URL the [`Router`] has
/// navigated to. This is always reset when triggering a navigation from the outside.
///
/// If the internal [`HistoryProvider`] doesn't support external navigation targets (e.g. the
/// [`MemoryHistoryProvider`]), application developers can handle the external navigation. The
/// router may render an incomplete page if this is not done.
///
/// [`MemoryHistoryProvider`]: crate::history::MemoryHistoryProvider
/// [`Router`]: crate::components::Router
#[derive(Clone)]
pub struct HistoryController {
    core: Arc<Mutex<ControlledHistoryCore>>,
}

impl HistoryController {
    /// Create a new [`HistoryController`] and a linked [`ControlledHistoryProvider`].
    #[must_use]
    pub fn new(internal: Box<dyn HistoryProvider>) -> (Self, ControlledHistoryProvider) {
        let core = Arc::new(Mutex::new(ControlledHistoryCore {
            callback: None,
            changed: false,
            external: None,
            history: internal,
        }));

        (
            Self { core: core.clone() },
            ControlledHistoryProvider { core },
        )
    }

    /// Get the external URL the router has navigated to.
    ///
    /// [`None`] if the router hasn't navigated to an external URL.
    #[must_use]
    pub fn get_external(&self) -> Option<String> {
        self.core.lock().unwrap().external.clone()
    }

    /// Check if the linked [`ControlledHistoryProvider`] has triggered a navigation since the last
    /// navigation triggered by the [`HistoryController`].
    #[must_use]
    pub fn has_redirected(&self) -> bool {
        self.core.lock().unwrap().changed
    }
}

impl HistoryProvider for HistoryController {
    fn current_path(&self) -> String {
        self.core.lock().unwrap().history.current_path()
    }

    fn current_prefix(&self) -> String {
        self.core.lock().unwrap().history.current_prefix()
    }

    fn current_query(&self) -> Option<String> {
        self.core.lock().unwrap().history.current_query()
    }

    fn can_go_back(&self) -> bool {
        self.core.lock().unwrap().history.can_go_back()
    }

    fn can_go_forward(&self) -> bool {
        self.core.lock().unwrap().history.can_go_forward()
    }

    fn go_back(&mut self) {
        let mut core = self.core.lock().unwrap();

        core.changed = false;
        core.external = None;
        core.history.go_back();

        if let Some(callback) = &core.callback {
            callback()
        }
    }

    fn go_forward(&mut self) {
        let mut core = self.core.lock().unwrap();

        core.changed = false;
        core.external = None;
        core.history.go_forward();

        if let Some(callback) = &core.callback {
            callback()
        }
    }

    fn push(&mut self, path: String) {
        let mut core = self.core.lock().unwrap();

        core.changed = false;
        core.external = None;
        core.history.push(path);

        if let Some(callback) = &core.callback {
            callback()
        }
    }

    fn replace(&mut self, path: String) {
        let mut core = self.core.lock().unwrap();

        core.changed = false;
        core.external = None;
        core.history.replace(path);

        if let Some(callback) = &core.callback {
            callback()
        }
    }

    fn can_external(&self) -> bool {
        self.core.lock().unwrap().history.can_external()
    }

    fn external(&self, url: String) {
        let mut core = self.core.lock().unwrap();

        core.changed = false;
        core.external = Some(url.clone());
        if core.history.can_external() {
            core.history.external(url);
        }

        if let Some(callback) = &core.callback {
            callback()
        }
    }
}

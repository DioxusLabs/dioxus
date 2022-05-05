use std::sync::{Arc, Mutex};

use super::HistoryProvider;

struct ControlledHistoryCore {
    callback: Option<Arc<dyn Fn() + Send + Sync>>,
    changed: bool,
    history: Box<dyn HistoryProvider>,
}

/// A [`HistoryProvider`] that can be controlled by a [`HistoryController`].
///
/// This can be used to control a [`Router`] from the outside by passing it in via the `history`
/// prop.
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
        core.history.go_back();
        core.changed = true;
    }

    fn go_forward(&mut self) {
        let mut core = self.core.lock().unwrap();
        core.history.go_forward();
        core.changed = true;
    }

    fn push(&mut self, path: String) {
        let mut core = self.core.lock().unwrap();
        core.history.push(path);
        core.changed = true;
    }

    fn replace(&mut self, path: String) {
        let mut core = self.core.lock().unwrap();
        core.history.replace(path);
        core.changed = true;
    }
}

/// A [`HistoryProvider`] that controls a [`ControlledHistoryProvider`].
///
/// This can be used to control a [`Router`] from the outside by passing in a linked
/// [`ControlledHistoryProvider`] via the `history` prop.
///
/// The [`HistoryController`] also implements [`HistoryProvider`] and causes the router to update
/// when the path is changed.
///
/// The [`HistoryController::has_redirected`] method can be used to check if the [`Router`] has
/// changed the current path/query. This is always reset when triggering a navigation from the
/// outside.
///
/// [`Router`]: crate::components::Router
#[derive(Clone)]
pub struct HistoryController {
    core: Arc<Mutex<ControlledHistoryCore>>,
}

impl HistoryController {
    /// Create a new [`HistoryController`] and a linked [`ControlledHistoryProvider`].
    pub fn new(internal: Box<dyn HistoryProvider>) -> (Self, ControlledHistoryProvider) {
        let core = Arc::new(Mutex::new(ControlledHistoryCore {
            callback: None,
            changed: false,
            history: internal,
        }));

        (
            Self { core: core.clone() },
            ControlledHistoryProvider { core },
        )
    }

    /// Check if the linked [`ControlledHistoryProvider`] has triggered a navigation since the last
    /// navigation triggered by the [`HistoryController`].
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
        core.history.go_back();
        core.changed = false;
        if let Some(callback) = &core.callback {
            callback()
        }
    }

    fn go_forward(&mut self) {
        let mut core = self.core.lock().unwrap();
        core.history.go_forward();
        core.changed = false;
        if let Some(callback) = &core.callback {
            callback()
        }
    }

    fn push(&mut self, path: String) {
        let mut core = self.core.lock().unwrap();
        core.history.push(path);
        core.changed = false;
        if let Some(callback) = &core.callback {
            callback()
        }
    }

    fn replace(&mut self, path: String) {
        let mut core = self.core.lock().unwrap();
        core.history.replace(path);
        core.changed = false;
        if let Some(callback) = &core.callback {
            callback()
        }
    }
}

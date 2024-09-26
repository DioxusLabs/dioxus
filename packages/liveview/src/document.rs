use crate::query::{Query, QueryEngine};
use dioxus_core::ScopeId;
use dioxus_document::{Document, EvalError};
use std::{
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, RwLock},
};

/// Provides the LiveviewDocument through [`ScopeId::provide_context`].
pub fn init_eval() {
    let query = ScopeId::ROOT.consume_context::<QueryEngine>().unwrap();
    let provider: Rc<dyn Document> = Rc::new(LiveviewDocument {
        query,
        action_tx: todo!(),
        updater_callback: todo!(),
        current_index: todo!(),
        routes: todo!(),
    });
    ScopeId::ROOT.provide_context(provider);
}

// The document impl for LiveView
pub struct LiveviewDocument {
    query: QueryEngine,
    action_tx: tokio::sync::mpsc::UnboundedSender<Action>,
    updater_callback: Arc<RwLock<Arc<dyn Fn() + Send + Sync>>>,
    current_index: usize,
    routes: BTreeMap<usize, String>,
}

impl Document for LiveviewDocument {
    fn eval(&self, js: String) -> dioxus_document::Eval {
        todo!()
    }

    fn set_title(&self, title: String) {
        _ = self.eval(format!("window.document.title = '{}';", title));
    }

    fn create_head_element(
        &self,
        name: &str,
        attributes: Vec<(&str, String)>,
        contents: Option<String>,
    ) {
        todo!()
    }

    fn go_back(&self) {
        let _ = self.action_tx.send(Action::GoBack);
    }

    fn go_forward(&self) {
        let _ = self.action_tx.send(Action::GoForward);
    }

    fn push_route(&self, route: String) {
        let _ = self.action_tx.send(Action::Push(route));
    }

    fn replace_route(&self, route: String) {
        let _ = self.action_tx.send(Action::Replace(route));
    }

    fn navigate_external(&self, url: String) -> bool {
        let _ = self.action_tx.send(Action::External(url));
        true
    }

    fn current_route(&self) -> String {
        self.routes[&self.current_index].clone()
    }

    fn can_go_back(&self) -> bool {
        // Check if the one before is contiguous (i.e., not an external page)
        let visited_indices: Vec<usize> = self.routes.keys().cloned().collect();
        visited_indices
            .iter()
            .position(|&rhs| self.current_index == rhs)
            .map_or(false, |index| {
                index > 0 && visited_indices[index - 1] == self.current_index - 1
            })
    }

    fn can_go_forward(&self) -> bool {
        // let timeline = self.timeline.lock().expect("unpoisoned mutex");
        // Check if the one after is contiguous (i.e., not an external page)
        let visited_indices: Vec<usize> = self.routes.keys().cloned().collect();
        visited_indices
            .iter()
            .rposition(|&rhs| self.current_index == rhs)
            .map_or(false, |index| {
                index < visited_indices.len() - 1
                    && visited_indices[index + 1] == self.current_index + 1
            })
    }

    fn updater(&self, callback: Arc<dyn Fn()>) {
        todo!()
        // let mut updater_callback = self.updater_callback.write().unwrap();
        // *updater_callback = callback;
    }
}

enum Action {
    GoBack,
    GoForward,
    Push(String),
    Replace(String),
    External(String),
}

use std::default;
use std::rc::Weak;
use webbrowser::Browser::Default;
use dioxus_document::{Document, Eval, EvalError, Evaluator};
use dioxus_html::track::default;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};

use crate::{query::Query, DesktopContext, WeakDesktopContext};

/// Code for the Dioxus channel used to communicate between the dioxus and javascript code
pub const NATIVE_EVAL_JS: &str = include_str!("./js/native_eval.js");

/// Represents the desktop-target's provider of evaluators.
pub struct DesktopDocument {
    pub(crate) desktop_ctx: WeakDesktopContext,
}

impl DesktopDocument {
    pub fn new(desktop_ctx: WeakDesktopContext) -> Self {
        Self { desktop_ctx }
    }
}

impl Document for DesktopDocument {
    fn eval(&self, js: String) -> Eval {
        Eval::new(DesktopEvaluator::create(self.desktop_ctx.clone(), js))
    }

    fn set_title(&self, title: String) {
        if let Some(desktop_ctx) = self.desktop_ctx.upgrade() {
            desktop_ctx.set_title(&title);
        }
    }
}

/// Represents a desktop-target's JavaScript evaluator.
pub(crate) struct DesktopEvaluator {
    query: Query<serde_json::Value>,
}

impl DesktopEvaluator {
    /// Creates a new evaluator for desktop-based targets.
    pub fn create(weak_desktop_ctx: WeakDesktopContext, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        let desktop_ctx = weak_desktop_ctx.upgrade().unwrap(); // todo: implement error or Default
        let query = desktop_ctx.query.new_query(&js, weak_desktop_ctx);

        // We create a generational box that is owned by the query slot so that when we drop the query slot, the generational box is also dropped.
        let owner = UnsyncStorage::owner();
        let query_id = query.id;
        let query = owner.insert(Box::new(DesktopEvaluator { query }) as Box<dyn Evaluator>);
        desktop_ctx.query.active_requests.slab.borrow_mut()[query_id].owner = Some(owner);

        query
    }
}

impl Evaluator for DesktopEvaluator {
    fn poll_join(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.query
            .poll_result(cx)
            .map_err(|e| EvalError::Communication(e.to_string()))
    }

    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        if let Err(e) = self.query.send(data) {
            return Err(EvalError::Communication(e.to_string()));
        }
        Ok(())
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    fn poll_recv(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.query
            .poll_recv(cx)
            .map_err(|e| EvalError::Communication(e.to_string()))
    }
}

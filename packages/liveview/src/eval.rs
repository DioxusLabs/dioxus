use dioxus_core::ScopeId;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator};
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use std::rc::Rc;

use crate::query::{Query, QueryEngine};

/// Provides the DesktopEvalProvider through [`cx.provide_context`].
pub fn init_eval() {
    let query = ScopeId::ROOT.consume_context::<QueryEngine>().unwrap();
    let provider: Rc<dyn EvalProvider> = Rc::new(DesktopEvalProvider { query });
    ScopeId::ROOT.provide_context(provider);
}

/// Reprints the desktop-target's provider of evaluators.
pub struct DesktopEvalProvider {
    query: QueryEngine,
}

impl EvalProvider for DesktopEvalProvider {
    fn new_evaluator(&self, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        DesktopEvaluator::create(self.query.clone(), js)
    }
}

/// Reprents a desktop-target's JavaScript evaluator.
pub(crate) struct DesktopEvaluator {
    query: Query<serde_json::Value>,
}

impl DesktopEvaluator {
    /// Creates a new evaluator for desktop-based targets.
    pub fn create(query_engine: QueryEngine, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        let query = query_engine.new_query(&js);
        // We create a generational box that is owned by the query slot so that when we drop the query slot, the generational box is also dropped.
        let owner = UnsyncStorage::owner();
        let query_id = query.id;
        let query = owner.insert(Box::new(DesktopEvaluator { query }) as Box<dyn Evaluator>);
        query_engine.active_requests.slab.borrow_mut()[query_id].owner = Some(owner);

        query
    }
}

impl Evaluator for DesktopEvaluator {
    /// # Panics
    /// This will panic if the query is currently being awaited.
    fn poll_join(
        &mut self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.query
            .poll_result(context)
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
    ///
    /// # Panics
    /// This will panic if the query is currently being awaited.
    fn poll_recv(
        &mut self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.query
            .poll_recv(context)
            .map_err(|e| EvalError::Communication(e.to_string()))
    }
}

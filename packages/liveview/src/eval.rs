use dioxus_core::ScopeId;
use dioxus_html::document::{Document, EvalError, Evaluator};
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use std::rc::Rc;

use crate::query::{Query, QueryEngine};

/// Provides the LiveviewDocument through [`ScopeId::provide_context`].
pub fn init_eval() {
    let query = ScopeId::ROOT.consume_context::<QueryEngine>().unwrap();
    let provider: Rc<dyn Document> = Rc::new(LiveviewDocument { query });
    ScopeId::ROOT.provide_context(provider);
}

/// Reprints the liveview-target's provider of evaluators.
pub struct LiveviewDocument {
    query: QueryEngine,
}

impl Document for LiveviewDocument {
    fn new_evaluator(&self, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        LiveviewEvaluator::create(self.query.clone(), js)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Represents a liveview-target's JavaScript evaluator.
pub(crate) struct LiveviewEvaluator {
    query: Query<serde_json::Value>,
}

impl LiveviewEvaluator {
    /// Creates a new evaluator for liveview-based targets.
    pub fn create(query_engine: QueryEngine, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        let query = query_engine.new_query(&js);
        // We create a generational box that is owned by the query slot so that when we drop the query slot, the generational box is also dropped.
        let owner = UnsyncStorage::owner();
        let query_id = query.id;
        let query = owner.insert(Box::new(LiveviewEvaluator { query }) as Box<dyn Evaluator>);
        query_engine.active_requests.slab.borrow_mut()[query_id].owner = Some(owner);

        query
    }
}

impl Evaluator for LiveviewEvaluator {
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

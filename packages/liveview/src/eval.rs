#![allow(clippy::await_holding_refcell_ref)]

use async_trait::async_trait;
use dioxus_core::ScopeState;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator};
use std::{cell::RefCell, rc::Rc};

use crate::query::{Query, QueryEngine};

/// Provides the DesktopEvalProvider through [`cx.provide_context`].
pub fn init_eval(cx: &ScopeState) {
    let query = cx.consume_context::<QueryEngine>().unwrap();
    let provider: Rc<dyn EvalProvider> = Rc::new(DesktopEvalProvider { query });
    cx.provide_context(provider);
}

/// Reprents the desktop-target's provider of evaluators.
pub struct DesktopEvalProvider {
    query: QueryEngine,
}

impl EvalProvider for DesktopEvalProvider {
    fn new_evaluator(&self, js: String) -> Result<Rc<dyn Evaluator>, EvalError> {
        Ok(Rc::new(DesktopEvaluator::new(self.query.clone(), js)))
    }
}

/// Reprents a desktop-target's JavaScript evaluator.
pub(crate) struct DesktopEvaluator {
    query: Rc<RefCell<Query<serde_json::Value>>>,
}

impl DesktopEvaluator {
    /// Creates a new evaluator for desktop-based targets.
    pub fn new(query: QueryEngine, js: String) -> Self {
        let query = query.new_query(&js);

        Self {
            query: Rc::new(RefCell::new(query)),
        }
    }
}

#[async_trait(?Send)]
impl Evaluator for DesktopEvaluator {
    /// # Panics
    /// This will panic if the query is currently being awaited.
    async fn join(&self) -> Result<serde_json::Value, EvalError> {
        self.query
            .borrow_mut()
            .result()
            .await
            .map_err(|e| EvalError::Communication(e.to_string()))
    }

    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        if let Err(e) = self.query.borrow_mut().send(data) {
            return Err(EvalError::Communication(e.to_string()));
        }
        Ok(())
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    ///
    /// # Panics
    /// This will panic if the query is currently being awaited.
    async fn recv(&self) -> Result<serde_json::Value, EvalError> {
        self.query
            .borrow_mut()
            .recv()
            .await
            .map_err(|e| EvalError::Communication(e.to_string()))
    }
}

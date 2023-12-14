use async_trait::async_trait;
use dioxus_core::ScopeState;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator};
use std::rc::Rc;

/// Provides the SSREvalProvider through [`cx.provide_context`].
pub fn init_eval(cx: &ScopeState) {
    let provider: Rc<dyn EvalProvider> = Rc::new(SSREvalProvider {});
    cx.provide_context(provider);
}

/// Reprents the ssr-target's provider of evaluators.
pub struct SSREvalProvider;
impl EvalProvider for SSREvalProvider {
    fn new_evaluator(&self, _: String) -> Result<Rc<dyn Evaluator>, EvalError> {
        Ok(Rc::new(SSREvaluator) as Rc<dyn Evaluator + 'static>)
    }
}

/// Represents a ssr-target's JavaScript evaluator.
pub struct SSREvaluator;

// In ssr rendering we never run or resolve evals.
#[async_trait(?Send)]
impl Evaluator for SSREvaluator {
    /// Runs the evaluated JavaScript.
    async fn join(&self) -> Result<serde_json::Value, EvalError> {
        std::future::pending::<()>().await;
        unreachable!()
    }

    /// Sends a message to the evaluated JavaScript.
    fn send(&self, _el: serde_json::Value) -> Result<(), EvalError> {
        Ok(())
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    async fn recv(&self) -> Result<serde_json::Value, EvalError> {
        std::future::pending::<()>().await;
        unreachable!()
    }
}

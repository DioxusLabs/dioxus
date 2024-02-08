use async_trait::async_trait;
use dioxus_core::ScopeId;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator};
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use std::rc::Rc;

/// Provides the SSREvalProvider through [`cx.provide_context`].
pub fn init_eval() {
    let provider: Rc<dyn EvalProvider> = Rc::new(SSREvalProvider {});
    ScopeId::ROOT.provide_context(provider);
}

/// Reprents the ssr-target's provider of evaluators.
pub struct SSREvalProvider;
impl EvalProvider for SSREvalProvider {
    fn new_evaluator(&self, _: String) -> Result<GenerationalBox<Box<dyn Evaluator>>, EvalError> {
        let owner = UnsyncStorage::owner();
        Ok(owner.insert(Box::new(SSREvaluator) as Box<dyn Evaluator + 'static>))
    }
}

/// Represents a ssr-target's JavaScript evaluator.
pub struct SSREvaluator;

// In ssr rendering we never run or resolve evals.
#[async_trait(?Send)]
impl Evaluator for SSREvaluator {
    /// Sends a message to the evaluated JavaScript.
    fn send(&self, _el: serde_json::Value) -> Result<(), EvalError> {
        Ok(())
    }

    fn poll_recv(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        std::task::Poll::Pending
    }

    fn poll_join(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        std::task::Poll::Pending
    }
}

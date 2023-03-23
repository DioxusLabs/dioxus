use std::rc::Rc;

use crate::query::Query;
use crate::query::QueryError;
use crate::use_window;
use dioxus_core::ScopeState;
use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;

/// A future that resolves to the result of a JavaScript evaluation.
pub struct EvalResult {
    pub(crate) query: Query<serde_json::Value>,
}

impl EvalResult {
    pub(crate) fn new(query: Query<serde_json::Value>) -> Self {
        Self { query }
    }
}

impl IntoFuture for EvalResult {
    type Output = Result<serde_json::Value, QueryError>;

    type IntoFuture = Pin<Box<dyn Future<Output = Result<serde_json::Value, QueryError>>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.query.resolve())
            as Pin<Box<dyn Future<Output = Result<serde_json::Value, QueryError>>>>
    }
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_eval(cx: &ScopeState) -> &Rc<dyn Fn(String) -> EvalResult> {
    let desktop = use_window(cx);
    &*cx.use_hook(|| {
        let desktop = desktop.clone();

        Rc::new(move |script: String| desktop.eval(&script)) as Rc<dyn Fn(String) -> EvalResult>
    })
}

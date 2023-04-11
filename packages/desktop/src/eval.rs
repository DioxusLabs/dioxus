use std::rc::Rc;

use crate::use_window;
use dioxus_core::ScopeState;
use serde::de::Error;
use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;

/// A future that resolves to the result of a JavaScript evaluation.
pub struct EvalResult {
    pub(crate) broadcast: tokio::sync::broadcast::Sender<serde_json::Value>,
}

impl EvalResult {
    pub(crate) fn new(sender: tokio::sync::broadcast::Sender<serde_json::Value>) -> Self {
        Self { broadcast: sender }
    }
}

impl IntoFuture for EvalResult {
    type Output = Result<serde_json::Value, serde_json::Error>;

    type IntoFuture = Pin<Box<dyn Future<Output = Result<serde_json::Value, serde_json::Error>>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let mut reciever = self.broadcast.subscribe();
            match reciever.recv().await {
                Ok(result) => Ok(result),
                Err(_) => Err(serde_json::Error::custom("No result returned")),
            }
        }) as Pin<Box<dyn Future<Output = Result<serde_json::Value, serde_json::Error>>>>
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

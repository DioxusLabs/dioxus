use async_trait::async_trait;
use dioxus_core::ScopeState;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator};
use std::rc::Rc;

use crate::{query::Query, DesktopContext};

/// Provides the DesktopEvalProvider through [`cx.provide_context`].
pub fn init_eval(cx: &ScopeState) {
    let provider: Rc<dyn EvalProvider> = Rc::new(DesktopEvalProvider {});
    cx.provide_context(provider);
}

/// Reprents the desktop-target's provider of evaluators.
pub struct DesktopEvalProvider;
impl EvalProvider for DesktopEvalProvider {
    fn new_evaluator(&self, cx: &ScopeState, js: String) -> Box<dyn Evaluator> {
        let desktop = cx.consume_context::<DesktopContext>().unwrap();
        Box::new(DesktopEvaluator::new(desktop, js))
    }
}

/// Reprents a desktop-target's JavaScript evaluator.
pub struct DesktopEvaluator {
    desktop_ctx: DesktopContext,
    query: Query<serde_json::Value>,
}

const DIOXUS_CODE: &str = r#"
    let dioxus = {
        recv: function () {
            return new Promise((resolve, _reject) => {
                // Ever 50 ms check for new data
                let timeout = setTimeout(() => {
                let msg = null;
                while (true) {
                    let data = _message_queue.shift();
                    if (data) {
                        msg = data;
                        break;
                    }
                }
                clearTimeout(timeout);
                resolve(msg);
                }, 50);
            });
        },

        send: function (value) {
            window.ipc.postMessage(
                JSON.stringify({
                    "method":"query",
                    "params": {
                        "id": _request_id,
                        "data": value,
                    }
                })
            );
        }
    }
    "#;

impl DesktopEvaluator {
    /// Creates a new evaluator for desktop-based targets.
    pub fn new(desktop_ctx: DesktopContext, js: String) -> Self {
        let code = format!(
            r#"
            {DIOXUS_CODE}

            new Promise(async (resolve, _reject) => {{
                {js}
                resolve(null);
            }});
            "#
        );

        let query = desktop_ctx
            .query
            .new_query_with_comm(&code, &desktop_ctx.webview);

        Self { desktop_ctx, query }
    }
}

#[async_trait(?Send)]
impl Evaluator for DesktopEvaluator {
    /// Runs the evaluated JavaScript.
    fn run(&mut self) -> Result<(), EvalError> {
        Ok(())
    }

    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        if let Err(e) = self.query.send(&self.desktop_ctx.webview, data) {
            return Err(EvalError::Communication(e.to_string()));
        }
        Ok(())
    }

    /// Receives a message from the evaluated JavaScript.
    async fn recv(&mut self) -> Result<serde_json::Value, EvalError> {
        match self.query.recv().await {
            Ok(d) => Ok(d),
            Err(e) => Err(EvalError::Communication(e.to_string())),
        }
    }

    /// Cleans up evaluation artifacts
    fn done(&mut self) {
        self.query.cleanup(Some(&self.desktop_ctx.webview));
    }
}

use async_trait::async_trait;
use dioxus_core::ScopeState;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator};
use std::cell::OnceCell;
use std::rc::Rc;

use crate::{query::Query, DesktopContext};

/// Provides the DesktopEvalProvider through [`cx.provide_context`].
pub fn init_eval(cx: &ScopeState) {
    let desktop_ctx = cx.consume_context::<DesktopContext>().unwrap();
    let provider: Rc<dyn EvalProvider> = Rc::new(DesktopEvalProvider { desktop_ctx });
    cx.provide_context(provider);
}

/// Reprents the desktop-target's provider of evaluators.
pub struct DesktopEvalProvider {
    desktop_ctx: DesktopContext,
}

impl EvalProvider for DesktopEvalProvider {
    fn new_evaluator(&self, js: String) -> Rc<dyn Evaluator> {
        Rc::new(DesktopEvaluator::new(self.desktop_ctx.clone(), js))
    }
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

/// Reprents a desktop-target's JavaScript evaluator.
pub struct DesktopEvaluator {
    desktop_ctx: DesktopContext,
    query: OnceCell<Query<serde_json::Value>>,
    sender: async_channel::Sender<serde_json::Value>,
    receiver: async_channel::Receiver<serde_json::Value>,
    code: String,
}

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

        let (sender, receiver) = async_channel::unbounded();

        Self {
            desktop_ctx,
            query: OnceCell::new(),
            sender,
            receiver,
            code,
        }
    }
}

#[async_trait(?Send)]
impl Evaluator for DesktopEvaluator {
    /// Runs the evaluated JavaScript.
    fn run(&self) -> Result<(), EvalError> {
        let desktop_ctx = &self.desktop_ctx;

        let query = desktop_ctx.query.new_query_with_comm(
            &self.code,
            &desktop_ctx.webview,
            self.sender.clone(),
        );

        self.query.set(query);

        Ok(())
    }

    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        if let Some(query) = self.query.get() {
            if let Err(e) = query.send(&self.desktop_ctx.webview, data) {
                return Err(EvalError::Communication(e.to_string()));
            }
            Ok(())
        } else {
            Err(EvalError::NotRan)
        }
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    async fn recv(&self) -> Result<serde_json::Value, EvalError> {
        self.receiver
            .recv()
            .await
            .map_err(|e| EvalError::Communication(e.to_string()))
    }
}

impl Drop for DesktopEvaluator {
    fn drop(&mut self) {
        if let Some(query) = &mut self.query.take() {
            query.cleanup(Some(&self.desktop_ctx.webview));
        }
    }
}

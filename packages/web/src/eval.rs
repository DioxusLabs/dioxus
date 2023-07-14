use async_trait::async_trait;
use dioxus_core::ScopeState;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator};
use js_sys::Function;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// Provides the WebEvalProvider through [`cx.provide_context`].
pub fn init_eval(cx: &ScopeState) {
    let provider: Rc<dyn EvalProvider> = Rc::new(WebEvalProvider {});
    cx.provide_context(provider);
}

/// Reprents the web-target's provider of evaluators.
pub struct WebEvalProvider;
impl EvalProvider for WebEvalProvider {
    fn new_evaluator(&self, js: String) -> Box<dyn Evaluator> {
        Box::new(WebEvaluator::new(js))
    }
}

/// Required to avoid blocking the Rust WASM thread.
const PROMISE_WRAPPER: &str = r#"
    return new Promise(async (resolve, _reject) => {
        {JS_CODE}
        resolve(null);
    });
    "#;

/// Reprents a web-target's JavaScript evaluator.
pub struct WebEvaluator {
    dioxus: Dioxus,
    receiver: async_channel::Receiver<serde_json::Value>,
    code: String,
    ran: bool,
}

impl WebEvaluator {
    /// Creates a new evaluator for web-based targets.
    pub fn new(js: String) -> Self {
        let (sender, receiver) = async_channel::unbounded();

        // This Rc cloning mess hurts but it seems to work..
        let a = Closure::<dyn FnMut(JsValue)>::new(move |data| {
            match serde_wasm_bindgen::from_value::<serde_json::Value>(data) {
                Ok(data) => _ = sender.send_blocking(data),
                Err(e) => {
                    // Can't really do much here.
                    log::error!("failed to serialize JsValue to serde_json::Value (eval communication) - {}", e.to_string());
                }
            }
        });

        let dioxus = Dioxus::new(a.as_ref().unchecked_ref());
        a.forget();

        // Wrap the evaluated JS in a promise so that wasm can continue running (send/receive data from js)
        let code = PROMISE_WRAPPER.replace("{JS_CODE}", &js);

        Self {
            dioxus,
            receiver,
            code,
            ran: false,
        }
    }
}

#[async_trait(?Send)]
impl Evaluator for WebEvaluator {
    /// Runs the evaluated JavaScript.
    fn run(&mut self) -> Result<(), EvalError> {
        if let Err(e) =
            Function::new_with_args("dioxus", &self.code).call1(&JsValue::NULL, &self.dioxus)
        {
            return Err(EvalError::InvalidJs(
                e.as_string().unwrap_or("unknown".to_string()),
            ));
        }

        self.ran = true;
        Ok(())
    }

    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        let data = match serde_wasm_bindgen::to_value::<serde_json::Value>(&data) {
            Ok(d) => d,
            Err(e) => return Err(EvalError::Communication(e.to_string())),
        };

        self.dioxus.rustSend(data);
        Ok(())
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    fn receiver(&mut self) -> async_channel::Receiver<serde_json::Value> {
        self.receiver.clone()
    }

    // No cleanup required
    fn done(&mut self) {}
}

#[wasm_bindgen(module = "/src/eval.js")]
extern "C" {
    pub type Dioxus;

    #[wasm_bindgen(constructor)]
    pub fn new(recv_callback: &Function) -> Dioxus;

    #[wasm_bindgen(method)]
    pub fn rustSend(this: &Dioxus, data: JsValue);

}

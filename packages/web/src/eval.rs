use async_trait::async_trait;
use dioxus_core::ScopeState;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator};
use js_sys::Function;
use serde_json::Value;
use std::{cell::RefCell, rc::Rc, str::FromStr};
use wasm_bindgen::prelude::*;

/// Provides the WebEvalProvider through [`cx.provide_context`].
pub fn init_eval(cx: &ScopeState) {
    let provider: Rc<dyn EvalProvider> = Rc::new(WebEvalProvider {});
    cx.provide_context(provider);
}

/// Reprents the web-target's provider of evaluators.
pub struct WebEvalProvider;
impl EvalProvider for WebEvalProvider {
    fn new_evaluator(&self, js: String) -> Result<Rc<dyn Evaluator>, EvalError> {
        WebEvaluator::new(js).map(|eval| Rc::new(eval) as Rc<dyn Evaluator + 'static>)
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
    channel_receiver: async_channel::Receiver<serde_json::Value>,
    result: RefCell<Option<serde_json::Value>>,
}

impl WebEvaluator {
    /// Creates a new evaluator for web-based targets.
    pub fn new(js: String) -> Result<Self, EvalError> {
        let (channel_sender, channel_receiver) = async_channel::unbounded();

        // This Rc cloning mess hurts but it seems to work..
        let recv_value = Closure::<dyn FnMut(JsValue)>::new(move |data| {
            match serde_wasm_bindgen::from_value::<serde_json::Value>(data) {
                Ok(data) => _ = channel_sender.send_blocking(data),
                Err(e) => {
                    // Can't really do much here.
                    tracing::error!("failed to serialize JsValue to serde_json::Value (eval communication) - {}", e);
                }
            }
        });

        let dioxus = Dioxus::new(recv_value.as_ref().unchecked_ref());
        recv_value.forget();

        // Wrap the evaluated JS in a promise so that wasm can continue running (send/receive data from js)
        let code = PROMISE_WRAPPER.replace("{JS_CODE}", &js);

        let result = match Function::new_with_args("dioxus", &code).call1(&JsValue::NULL, &dioxus) {
            Ok(result) => {
                if let Ok(stringified) = js_sys::JSON::stringify(&result) {
                    if !stringified.is_undefined() && stringified.is_valid_utf16() {
                        let string: String = stringified.into();
                        Value::from_str(&string).map_err(|e| {
                            EvalError::Communication(format!("Failed to parse result - {}", e))
                        })?
                    } else {
                        return Err(EvalError::Communication(
                            "Failed to stringify result".into(),
                        ));
                    }
                } else {
                    return Err(EvalError::Communication(
                        "Failed to stringify result".into(),
                    ));
                }
            }
            Err(err) => {
                return Err(EvalError::InvalidJs(
                    err.as_string().unwrap_or("unknown".to_string()),
                ));
            }
        };

        Ok(Self {
            dioxus,
            channel_receiver,
            result: RefCell::new(Some(result)),
        })
    }
}

#[async_trait(?Send)]
impl Evaluator for WebEvaluator {
    /// Runs the evaluated JavaScript.
    async fn join(&self) -> Result<serde_json::Value, EvalError> {
        self.result.take().ok_or(EvalError::Finished)
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
    async fn recv(&self) -> Result<serde_json::Value, EvalError> {
        self.channel_receiver
            .recv()
            .await
            .map_err(|_| EvalError::Communication("failed to receive data from js".to_string()))
    }
}

#[wasm_bindgen(module = "/src/eval.js")]
extern "C" {
    pub type Dioxus;

    #[wasm_bindgen(constructor)]
    pub fn new(recv_callback: &Function) -> Dioxus;

    #[wasm_bindgen(method)]
    pub fn rustSend(this: &Dioxus, data: JsValue);
}

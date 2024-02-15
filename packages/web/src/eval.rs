use core::panic;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator};
use futures_util::StreamExt;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use js_sys::Function;
use serde_json::Value;
use std::{rc::Rc, str::FromStr};
use wasm_bindgen::prelude::*;

/// Provides the WebEvalProvider through [`cx.provide_context`].
pub fn init_eval() {
    let provider: Rc<dyn EvalProvider> = Rc::new(WebEvalProvider);
    dioxus_core::ScopeId::ROOT.provide_context(provider);
}

/// Represents the web-target's provider of evaluators.
pub struct WebEvalProvider;
impl EvalProvider for WebEvalProvider {
    fn new_evaluator(&self, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        WebEvaluator::create(js)
    }
}

/// Required to avoid blocking the Rust WASM thread.
const PROMISE_WRAPPER: &str = r#"
    return new Promise(async (resolve, _reject) => {
        {JS_CODE}
        resolve(null);
    });
    "#;

/// Represents a web-target's JavaScript evaluator.
struct WebEvaluator {
    dioxus: Dioxus,
    channel_receiver: futures_channel::mpsc::UnboundedReceiver<serde_json::Value>,
    result: Option<Result<serde_json::Value, EvalError>>,
}

impl WebEvaluator {
    /// Creates a new evaluator for web-based targets.
    fn create(js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        let (mut channel_sender, channel_receiver) = futures_channel::mpsc::unbounded();
        let owner = UnsyncStorage::owner();
        let invalid = owner.invalid();

        // This Rc cloning mess hurts but it seems to work..
        let recv_value = Closure::<dyn FnMut(JsValue)>::new(move |data| {
            // Drop the owner when the sender is dropped.
            let _ = &owner;
            match serde_wasm_bindgen::from_value::<serde_json::Value>(data) {
                Ok(data) => _ = channel_sender.start_send(data),
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
                        })
                    } else {
                        Err(EvalError::Communication(
                            "Failed to stringify result".into(),
                        ))
                    }
                } else {
                    Err(EvalError::Communication(
                        "Failed to stringify result".into(),
                    ))
                }
            }
            Err(err) => Err(EvalError::InvalidJs(
                err.as_string().unwrap_or("unknown".to_string()),
            )),
        };

        invalid.set(Box::new(Self {
            dioxus,
            channel_receiver,
            result: Some(result),
        }) as Box<dyn Evaluator + 'static>);

        invalid
    }
}

impl Evaluator for WebEvaluator {
    /// Runs the evaluated JavaScript.
    fn poll_join(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        if let Some(result) = self.result.take() {
            std::task::Poll::Ready(result)
        } else {
            std::task::Poll::Ready(Err(EvalError::Finished))
        }
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
    fn poll_recv(
        &mut self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.channel_receiver.poll_next_unpin(context).map(|poll| {
            poll.ok_or_else(|| {
                EvalError::Communication("failed to receive data from js".to_string())
            })
        })
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

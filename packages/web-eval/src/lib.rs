//! Web-based JavaScript evaluator for Dioxus.
//!
//! This crate provides the `WebEvaluator` which can be used by both
//! the web renderer (via wasm-bindgen) and the desktop renderer (via wry-bindgen).

use dioxus_document::{EvalError, Evaluator};
use futures_util::FutureExt;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use js_sys::Function;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::result;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// A wrapper that allows Rust ownership to be transferred to JavaScript.
/// When the JavaScript side drops the channel, this owner will be dropped,
/// cleaning up the associated Rust resources.
#[wasm_bindgen]
pub struct JSOwner {
    _owner: Box<dyn std::any::Any>,
}

impl JSOwner {
    /// Create a new JSOwner that wraps the given value.
    pub fn new(owner: impl std::any::Any) -> Self {
        Self {
            _owner: Box::new(owner),
        }
    }
}

#[wasm_bindgen(module = "/src/js/eval.js")]
extern "C" {
    /// A weak reference to a DioxusChannel that can be used from Rust.
    pub type WeakDioxusChannel;

    /// Send data from Rust to JavaScript through the weak channel reference.
    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WeakDioxusChannel, value: wasm_bindgen::JsValue);

    /// Receive data sent from JavaScript in Rust through the weak channel reference.
    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WeakDioxusChannel) -> wasm_bindgen::JsValue;
}

#[wasm_bindgen(module = "/src/js/eval.js")]
extern "C" {
    /// A channel for bidirectional communication between Rust and JavaScript.
    pub type WebDioxusChannel;

    /// Create a new WebDioxusChannel with the given owner.
    #[wasm_bindgen(constructor)]
    pub fn new(owner: JSOwner) -> WebDioxusChannel;

    /// Send data from Rust to JavaScript.
    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WebDioxusChannel, value: wasm_bindgen::JsValue);

    /// Receive data sent from JavaScript in Rust.
    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WebDioxusChannel) -> wasm_bindgen::JsValue;

    /// Send data from JavaScript to Rust (called from JS side).
    #[wasm_bindgen(method)]
    pub fn send(this: &WebDioxusChannel, value: wasm_bindgen::JsValue);

    /// Receive data sent from Rust in JavaScript (called from JS side).
    #[wasm_bindgen(method)]
    pub async fn recv(this: &WebDioxusChannel) -> wasm_bindgen::JsValue;

    /// Get a weak reference to this channel.
    #[wasm_bindgen(method)]
    pub fn weak(this: &WebDioxusChannel) -> WeakDioxusChannel;
}

/// JavaScript wrapper that ensures async code doesn't block the Rust WASM thread.
/// The evaluated code is wrapped in an async IIFE that calls `dioxus.close()` when done.
const PROMISE_WRAPPER: &str = r#"
    return (async function(){
        {JS_CODE}

        dioxus.close();
    })();
"#;

type NextPoll = Pin<Box<dyn Future<Output = Result<serde_json::Value, EvalError>>>>;

/// A web-based JavaScript evaluator that uses wasm-bindgen for communication.
///
/// This evaluator works in both pure web (wasm32) contexts and in desktop
/// contexts using wry-bindgen (which patches wasm-bindgen for native webviews).
pub struct WebEvaluator {
    channels: WeakDioxusChannel,
    next_future: Option<NextPoll>,
    result: Pin<Box<dyn Future<Output = result::Result<Value, EvalError>>>>,
}

impl WebEvaluator {
    /// Creates a new evaluator and executes the given JavaScript code.
    ///
    /// The JavaScript code has access to a `dioxus` object with the following methods:
    /// - `dioxus.send(data)` - Send data to Rust
    /// - `dioxus.recv()` - Receive data from Rust (returns a Promise)
    /// - `dioxus.close()` - Close the channel (called automatically when the code finishes)
    ///
    /// The return value of the JavaScript code will be available via `poll_join`.
    pub fn create(js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        let owner = UnsyncStorage::owner();

        // Add the drop handler to DioxusChannel so that it gets dropped when the channel is dropped in JS
        let channels = WebDioxusChannel::new(JSOwner::new(owner.clone()));

        // The Rust side of the channel is a weak reference to the DioxusChannel
        let weak_channels = channels.weak();

        // Wrap the evaluated JS in a promise so that wasm can continue running (send/receive data from JS)
        let code = PROMISE_WRAPPER.replace("{JS_CODE}", &js);

        let result = match Function::new_with_args("dioxus", &code).call1(&JsValue::NULL, &channels)
        {
            Ok(result) => {
                let future = js_sys::Promise::resolve(&result);
                let js_future = JsFuture::from(future);
                Box::pin(async move {
                    let result = js_future.await.map_err(|e| {
                        EvalError::Communication(format!("Failed to await result - {:?}", e))
                    })?;
                    value_from_js_value(&result)
                })
                    as Pin<Box<dyn Future<Output = result::Result<Value, EvalError>>>>
            }
            Err(err) => Box::pin(futures_util::future::ready(Err(EvalError::InvalidJs(
                err.as_string().unwrap_or("unknown".to_string()),
            )))),
        };

        owner.insert(Box::new(Self {
            channels: weak_channels,
            result,
            next_future: None,
        }) as Box<dyn Evaluator>)
    }
}

impl Evaluator for WebEvaluator {
    /// Polls for the final result of the JavaScript evaluation.
    fn poll_join(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.result.poll_unpin(cx)
    }

    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        let data = value_to_js_value(&data)?;
        self.channels.rust_send(data);
        Ok(())
    }

    /// Polls for the next message from the evaluated JavaScript.
    fn poll_recv(
        &mut self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        if self.next_future.is_none() {
            let channels: WebDioxusChannel = self.channels.clone().into();
            let pinned = Box::pin(async move {
                let fut = channels.rust_recv();
                let data = fut.await;
                value_from_js_value(&data)
            });
            self.next_future = Some(pinned);
        }
        let fut = self.next_future.as_mut().unwrap();
        let mut pinned = std::pin::pin!(fut);
        let result = pinned.as_mut().poll(context);
        if result.is_ready() {
            self.next_future = None;
        }
        result
    }
}

/// We don't use serde-wasm-bindgen here because we need to make sure this works on desktop as well which
/// requires using wasm-bindgen-x instead of wasm-bindgen directly.
fn value_from_js_value<T: DeserializeOwned>(value: &JsValue) -> Result<T, EvalError> {
    let stringified = js_sys::JSON::stringify(value)
        .map_err(|e| EvalError::Communication(format!("Failed to stringify result - {:?}", e)))?;
    if !stringified.is_undefined() && stringified.is_valid_utf16() {
        let string: String = stringified.into();
        serde_json::de::from_str(&string)
            .map_err(|e| EvalError::Communication(format!("Failed to parse result - {}", e)))
    } else {
        Err(EvalError::Communication(
            "Failed to stringify result - undefined or not valid utf16".to_string(),
        ))
    }
}

fn value_to_js_value<T: Serialize>(value: &T) -> Result<JsValue, EvalError> {
    let json_string = serde_json::to_string(value)
        .map_err(|e| EvalError::Communication(format!("Failed to serialize value - {}", e)))?;
    js_sys::JSON::parse(&json_string)
        .map_err(|e| EvalError::Communication(format!("Failed to parse JSON string - {:?}", e)))
}

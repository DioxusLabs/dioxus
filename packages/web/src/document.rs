use dioxus_core::prelude::queue_effect;
use dioxus_core::ScopeId;
use dioxus_document::{
    create_element_in_head, Document, Eval, EvalError, Evaluator, LinkProps, MetaProps,
    ScriptProps, StyleProps,
};
use dioxus_history::History;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use js_sys::Function;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::{rc::Rc, str::FromStr};
use wasm_bindgen::prelude::*;

use crate::history::WebHistory;

#[wasm_bindgen::prelude::wasm_bindgen]
pub struct JSOwner {
    _owner: Box<dyn std::any::Any>,
}

impl JSOwner {
    pub fn new(owner: impl std::any::Any) -> Self {
        Self {
            _owner: Box::new(owner),
        }
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/js/eval.js")]
extern "C" {
    pub type WebDioxusChannel;

    #[wasm_bindgen(constructor)]
    pub fn new(owner: JSOwner) -> WebDioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WebDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WebDioxusChannel) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(method)]
    pub fn send(this: &WebDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method)]
    pub async fn recv(this: &WebDioxusChannel) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(method)]
    pub fn weak(this: &WebDioxusChannel) -> WeakDioxusChannel;

    pub type WeakDioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WeakDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WeakDioxusChannel) -> wasm_bindgen::JsValue;
}

/// Provides the Document through [`ScopeId::provide_context`].
pub fn init_document() {
    let provider: Rc<dyn Document> = Rc::new(WebDocument);
    if ScopeId::ROOT.has_context::<Rc<dyn Document>>().is_none() {
        ScopeId::ROOT.provide_context(provider);
    }
    let history_provider: Rc<dyn History> = Rc::new(WebHistory::default());
    if ScopeId::ROOT.has_context::<Rc<dyn History>>().is_none() {
        ScopeId::ROOT.provide_context(history_provider);
    }
}

/// The web-target's document provider.
#[derive(Clone)]
pub struct WebDocument;
impl Document for WebDocument {
    fn eval(&self, js: String) -> Eval {
        Eval::new(WebEvaluator::create(js))
    }

    /// Set the title of the document
    fn set_title(&self, title: String) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(format!("document.title = {title:?};"));
        });
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head("meta", &props.attributes(), None));
        });
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head(
                "script",
                &props.attributes(),
                props.script_contents().ok(),
            ));
        });
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head(
                "style",
                &props.attributes(),
                props.style_contents().ok(),
            ));
        });
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head("link", &props.attributes(), None));
        });
    }
}

/// Required to avoid blocking the Rust WASM thread.
const PROMISE_WRAPPER: &str = r#"
    return new Promise(async (resolve, _reject) => {
        {JS_CODE}
        resolve(null);
    });
"#;

type NextPoll = Pin<Box<dyn Future<Output = Result<serde_json::Value, EvalError>>>>;

/// Represents a web-target's JavaScript evaluator.
struct WebEvaluator {
    channels: WeakDioxusChannel,
    next_future: Option<NextPoll>,
    result: Option<Result<serde_json::Value, EvalError>>,
}

impl WebEvaluator {
    /// Creates a new evaluator for web-based targets.
    fn create(js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        let owner = UnsyncStorage::owner();

        // add the drop handler to DioxusChannel so that it gets dropped when the channel is dropped in js
        let channels = WebDioxusChannel::new(JSOwner::new(owner.clone()));

        // The Rust side of the channel is a weak reference to the DioxusChannel
        let weak_channels = channels.weak();

        // Wrap the evaluated JS in a promise so that wasm can continue running (send/receive data from js)
        let code = PROMISE_WRAPPER.replace("{JS_CODE}", &js);

        let result = match Function::new_with_args("dioxus", &code).call1(&JsValue::NULL, &channels)
        {
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

        owner.insert(Box::new(Self {
            channels: weak_channels,
            result: Some(result),
            next_future: None,
        }) as Box<dyn Evaluator>)
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
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        let data = match data.serialize(&serializer) {
            Ok(d) => d,
            Err(e) => return Err(EvalError::Communication(e.to_string())),
        };

        self.channels.rust_send(data);
        Ok(())
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    fn poll_recv(
        &mut self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        if self.next_future.is_none() {
            let channels: WebDioxusChannel = self.channels.clone().into();
            let pinned = Box::pin(async move {
                let fut = channels.rust_recv();
                let data = fut.await;
                serde_wasm_bindgen::from_value::<serde_json::Value>(data)
                    .map_err(|err| EvalError::Communication(err.to_string()))
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

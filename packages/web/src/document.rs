use dioxus_core::queue_effect;
use dioxus_core::ScopeId;
use dioxus_core::{provide_context, Runtime};
use dioxus_document::{
    Document, Eval, EvalError, Evaluator, LinkProps, MetaProps, ScriptProps, StyleProps,
};
use dioxus_history::History;
use futures_util::FutureExt;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use js_sys::Function;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::result;
use std::{rc::Rc, str::FromStr};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

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
    pub type WeakDioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WeakDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WeakDioxusChannel) -> wasm_bindgen::JsValue;
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

}

fn init_document_with(document: impl FnOnce(), history: impl FnOnce()) {
    use dioxus_core::has_context;
    Runtime::current().in_scope(ScopeId::ROOT, || {
        if has_context::<Rc<dyn Document>>().is_none() {
            document();
        }
        if has_context::<Rc<dyn History>>().is_none() {
            history();
        }
    })
}

/// Provides the Document through [`dioxus_core::provide_context`].
pub fn init_document() {
    // If hydrate is enabled, we add the FullstackWebDocument with the initial hydration data
    #[cfg(not(feature = "hydrate"))]
    {
        use dioxus_history::provide_history_context;

        init_document_with(
            || {
                provide_context(Rc::new(WebDocument) as Rc<dyn Document>);
            },
            || {
                provide_history_context(Rc::new(WebHistory::default()));
            },
        );
    }
}

#[cfg(feature = "hydrate")]
pub fn init_fullstack_document() {
    use dioxus_fullstack_core::{
        document::FullstackWebDocument, history::provide_fullstack_history_context,
    };

    init_document_with(
        || {
            provide_context(Rc::new(FullstackWebDocument::from(WebDocument)) as Rc<dyn Document>);
        },
        || provide_fullstack_history_context(WebHistory::default()),
    );
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
        queue_effect(move || {
            _ = append_element_to_head("meta", &props.attributes(), None);
        });
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        queue_effect(move || {
            _ = append_element_to_head(
                "script",
                &props.attributes(),
                props.script_contents().ok().as_deref(),
            );
        });
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        queue_effect(move || {
            _ = append_element_to_head(
                "style",
                &props.attributes(),
                props.style_contents().ok().as_deref(),
            );
        });
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        queue_effect(move || {
            _ = append_element_to_head("link", &props.attributes(), None);
        });
    }
}

fn append_element_to_head(
    local_name: &str,
    attributes: &Vec<(&'static str, String)>,
    text_content: Option<&str>,
) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let head = document.head().expect("document should have a head");

    let element = document.create_element(local_name)?;
    for (name, value) in attributes {
        element.set_attribute(name, value)?;
    }
    if text_content.is_some() {
        element.set_text_content(text_content);
    }
    head.append_child(&element)?;

    Ok(())
}

/// Required to avoid blocking the Rust WASM thread.
const PROMISE_WRAPPER: &str = r#"
    return (async function(){
        {JS_CODE}

        dioxus.close();
    })();
"#;

type NextPoll = Pin<Box<dyn Future<Output = Result<serde_json::Value, EvalError>>>>;

/// Represents a web-target's JavaScript evaluator.
struct WebEvaluator {
    channels: WeakDioxusChannel,
    next_future: Option<NextPoll>,
    result: Pin<Box<dyn Future<Output = result::Result<Value, EvalError>>>>,
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
                let future = js_sys::Promise::resolve(&result);
                let js_future = JsFuture::from(future);
                Box::pin(async move {
                    let result = js_future.await.map_err(|e| {
                        EvalError::Communication(format!("Failed to await result - {:?}", e))
                    })?;
                    let stringified = js_sys::JSON::stringify(&result).map_err(|e| {
                        EvalError::Communication(format!("Failed to stringify result - {:?}", e))
                    })?;
                    if !stringified.is_undefined() && stringified.is_valid_utf16() {
                        let string: String = stringified.into();
                        Value::from_str(&string).map_err(|e| {
                            EvalError::Communication(format!("Failed to parse result - {}", e))
                        })
                    } else {
                        Err(EvalError::Communication(
                            "Failed to stringify result - undefined or not valid utf16".to_string(),
                        ))
                    }
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
    /// Runs the evaluated JavaScript.
    fn poll_join(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.result.poll_unpin(cx)
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

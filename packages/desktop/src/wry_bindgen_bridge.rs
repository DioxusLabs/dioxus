//! wry-bindgen integration for dioxus desktop.
//!
//! This module provides the bridge between dioxus desktop and wry-bindgen,
//! enabling typed Rust<->JS communication through wry-bindgen's binary protocol.

use dioxus_core::Runtime;
use dioxus_html::HtmlEvent;
use std::rc::Rc;
use wry_bindgen::prelude::*;

/// Set up the wasm-bindgen event handler that JavaScript calls for DOM events.
///
/// The handler:
/// 1. Receives serialized event JSON from JavaScript
/// 2. Parses the event and sends it to the VirtualDom (same thread)
/// 3. Blocks until the VirtualDom processes the event
/// 4. Returns the preventDefault response as JSON
///
/// IMPORTANT: This must be called from the wry-bindgen thread (same thread as VirtualDom).
#[wasm_bindgen(crate = wry_bindgen, inline_js = r#"
export function setEventHandler(handler) {
    window.rustEventHandler = handler;
}
"#)]
extern "C" {
    fn setEventHandler(handler: &Closure<dyn FnMut(String) -> String>);
}

/// Initialize the event handler closure with the given event sender.
///
/// This should be called once during VirtualDom initialization on the wry-bindgen thread.
pub fn setup_event_handler(runtime: Rc<Runtime>) {
    let closure = Closure::new(move |event_json: String| -> String {
        handle_event_from_js(&runtime, event_json)
    });

    setEventHandler(&closure);

    // Keep the closure alive for the lifetime of the webview
    closure.forget();
}

/// A synchronous response to a browser event which may prevent the default browser's action
#[derive(serde::Serialize, Default)]
struct SynchronousEventResponse {
    #[serde(rename = "preventDefault")]
    prevent_default: bool,
}

impl SynchronousEventResponse {
    fn new(prevent_default: bool) -> Self {
        Self { prevent_default }
    }
}

/// Handle an event from JavaScript, returning the serialized response.
fn handle_event_from_js(runtime: &Rc<Runtime>, event_json: String) -> String {
    let response = match serde_json::from_str::<HtmlEvent>(&event_json) {
        Ok(event) => {
            let HtmlEvent {
                element,
                name,
                bubbles,
                data,
            } = event;

            // Convert to desktop-specific event types where needed
            let as_any = data.into_any();

            let event = dioxus_core::Event::new(as_any, bubbles);
            runtime.handle_event(&name, event.clone(), element);

            SynchronousEventResponse::new(!event.default_action_enabled())
        }
        Err(err) => {
            tracing::error!("Error parsing event from JavaScript: {:?}", err);
            SynchronousEventResponse::default()
        }
    };

    serde_json::to_string(&response).unwrap_or_default()
}

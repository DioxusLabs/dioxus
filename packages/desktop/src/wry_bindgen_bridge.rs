//! wry-bindgen integration for dioxus desktop.
//!
//! This module provides the bridge between dioxus desktop and wry-bindgen,
//! enabling typed Rust<->JS communication through wry-bindgen's binary protocol.

use dioxus_core::{ElementId, Runtime};
use dioxus_web_sys_events::virtual_event_from_websys_event;
use std::any::Any;
use std::rc::Rc;
use wry_bindgen::prelude::*;

#[wasm_bindgen(crate = wry_bindgen, inline_js = r#"
export function setEventHandler(handler) {
    window.rustEventHandler = handler;
}
export function setMountedHandler(handler) {
    window.rustMountedHandler = handler;
}
"#)]
extern "C" {
    fn setEventHandler(handler: Closure<dyn FnMut(web_sys::Event, String, u64, bool) -> bool>);
    fn setMountedHandler(handler: Closure<dyn FnMut(web_sys::Element, u64, bool)>);
}

/// Initialize the event handler closure with the given event sender.
///
/// This should be called once during VirtualDom initialization on the wry-bindgen thread.
/// The handler receives:
/// - event: The raw web_sys::Event
/// - name: The event name (e.g., "click", "input")
/// - element_id: The dioxus element ID
/// - bubbles: Whether the event bubbles
///
/// Returns true if preventDefault should be called.
pub fn setup_event_handler(runtime: Rc<Runtime>) {
    let runtime_clone = runtime.clone();
    let event_closure = Closure::new(
        move |event: web_sys::Event, name: String, element_id: u64, bubbles: bool| {
            handle_event_from_js(&runtime_clone, event, name, element_id, bubbles)
        },
    );

    let mounted_closure = Closure::new(
        move |element: web_sys::Element, element_id: u64, bubbles: bool| {
            handle_mounted_from_js(&runtime, element, element_id, bubbles)
        },
    );

    setEventHandler(event_closure);
    setMountedHandler(mounted_closure);
}

/// Handle an event from JavaScript, returning whether to prevent default.
fn handle_event_from_js(
    runtime: &Rc<Runtime>,
    event: web_sys::Event,
    name: String,
    element_id: u64,
    bubbles: bool,
) -> bool {
    use wry_bindgen::JsCast;

    // Get the target element for the event
    let target = event
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .unwrap_or_else(|| {
            dioxus_web_sys_events::load_document()
                .document_element()
                .expect("document should have a root element")
        });

    // Convert to platform event data using the shared web-sys-events crate
    let platform_event = virtual_event_from_websys_event(event, target);

    let element = ElementId(element_id as usize);
    let event = dioxus_core::Event::new(Rc::new(platform_event) as Rc<dyn Any>, bubbles);
    runtime.handle_event(&name, event.clone(), element);

    // Return true if we should prevent the default action
    !event.default_action_enabled()
}

/// Handle a mounted event from JavaScript.
fn handle_mounted_from_js(
    runtime: &Rc<Runtime>,
    element: web_sys::Element,
    element_id: u64,
    bubbles: bool,
) {
    use dioxus_html::PlatformEventData;

    // For mounted events, we pass the element directly as the event data
    let platform_event = PlatformEventData::new(Box::new(element));

    let element = ElementId(element_id as usize);
    let event = dioxus_core::Event::new(Rc::new(platform_event) as Rc<dyn Any>, bubbles);
    runtime.handle_event("mounted", event, element);
}

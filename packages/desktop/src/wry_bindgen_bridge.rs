//! wry-bindgen integration for dioxus desktop.
//!
//! This module provides the bridge between dioxus desktop and wry-bindgen,
//! enabling typed Rust<->JS communication through wry-bindgen's FFI mechanism.

use crate::file_upload::{DesktopFileDragEvent, NativeFileHover};
use dioxus_core::{ElementId, Runtime};
use dioxus_html::PlatformEventData;
use dioxus_web_sys_events::{Synthetic, virtual_event_from_websys_event};
use std::any::Any;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
export function setEventHandler(handler) {
    window.rustEventHandler = handler;
}
export function mountedElementById(id) {
    if (!window.interpreter) {
        return null;
    }
    const node = window.interpreter.getNode(id);
    return node instanceof Element ? node : null;
}
"#)]
extern "C" {
    fn setEventHandler(handler: Closure<dyn FnMut(web_sys::Event, String, u64, bool) -> bool>);
    fn mountedElementById(id: u64) -> Option<web_sys::Element>;
}

/// Initialize the event handler closures for the wry-bindgen bridge.
///
/// This should be called once during VirtualDom initialization on the DOM thread.
/// The handler receives:
/// - event: The raw web_sys::Event (wry-bindgen's web-sys)
/// - name: The event name (e.g., "click", "input")
/// - element_id: The dioxus element ID
/// - bubbles: Whether the event bubbles
///
/// Returns true if preventDefault should be called.
pub fn setup_event_handler(runtime: Rc<Runtime>, file_hover: NativeFileHover) {
    let runtime_clone = runtime.clone();
    let event_closure = Closure::new(
        move |event: web_sys::Event, name: String, element_id: u64, bubbles: bool| {
            handle_event_from_js(
                &runtime_clone,
                &file_hover,
                event,
                name,
                element_id,
                bubbles,
            )
        },
    );

    setEventHandler(event_closure);
}

/// Handle an event from JavaScript, returning whether to prevent default.
fn handle_event_from_js(
    runtime: &Rc<Runtime>,
    file_hover: &NativeFileHover,
    event: web_sys::Event,
    name: String,
    element_id: u64,
    bubbles: bool,
) -> bool {
    use wasm_bindgen::JsCast;

    // Drop events whose JS type doesn't match what the converters will unchecked-cast to
    // (e.g. a user-dispatched plain `Event` with a typed name like "keydown")
    if !dioxus_web_sys_events::event_type_matches(&name, &event) {
        return false;
    }

    // Get the target element for the event
    let target = event
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .unwrap_or_else(|| {
            dioxus_web_sys_events::load_document()
                .document_element()
                .expect("document should have a root element")
        });

    // For drag events, we need to inject native file paths from the file_hover context. A
    // JS-dispatched plain `Event` with a drag name (synthetic events are a supported pattern)
    // fails the `DragEvent` cast and falls back to the generic conversion instead of panicking.
    let platform_event: PlatformEventData = if dioxus_web_sys_events::is_drag_event(&name) {
        match event.dyn_into::<web_sys::DragEvent>() {
            Ok(drag_event) => {
                // Create a DesktopFileDragEvent with native file paths
                let desktop_drag = DesktopFileDragEvent::new(
                    Synthetic::new(drag_event),
                    file_hover.current_paths(),
                );
                PlatformEventData::new(Box::new(desktop_drag))
            }
            Err(event) => virtual_event_from_websys_event(event, target),
        }
    } else {
        // Convert to platform event data using the shared web-sys-events crate
        virtual_event_from_websys_event(event, target)
    };

    let element = ElementId(element_id as usize);
    let event = dioxus_core::Event::new(Rc::new(platform_event) as Rc<dyn Any>, bubbles);
    runtime.handle_event(&name, event.clone(), element);

    // Return true if we should prevent the default action
    !event.default_action_enabled()
}

/// Handle a mounted event after the webview has applied the edit batch that created the listener.
pub(crate) fn handle_mounted_event(runtime: &Rc<Runtime>, element_id: ElementId) {
    let Some(element) = mountedElementById(element_id.0 as u64) else {
        return;
    };

    dioxus_web_sys_events::dispatch_mounted_event(runtime, element_id, element);
}

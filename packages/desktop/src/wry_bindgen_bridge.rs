//! wry-bindgen integration for dioxus desktop.
//!
//! This module provides the bridge between dioxus desktop and wry-bindgen,
//! enabling typed Rust<->JS communication through wry-bindgen's FFI mechanism.

use crate::file_upload::{DesktopFileDragEvent, NativeFileHover};
use dioxus_core::{ElementId, Runtime};
use dioxus_html::PlatformEventData;
use dioxus_web_sys_events::{virtual_event_from_websys_event, Synthetic};
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
    fn setEventHandler(handler: Closure<dyn FnMut(web_sys_x::Event, String, u64, bool) -> bool>);
    fn setMountedHandler(handler: Closure<dyn FnMut(web_sys_x::Element, u64, bool)>);
}

/// Initialize the event handler closures for the wry-bindgen bridge.
///
/// This should be called once during VirtualDom initialization on the wry-bindgen thread.
/// The handler receives:
/// - event: The raw web_sys_x::Event (wry-bindgen's web-sys)
/// - name: The event name (e.g., "click", "input")
/// - element_id: The dioxus element ID
/// - bubbles: Whether the event bubbles
///
/// Returns true if preventDefault should be called.
pub fn setup_event_handler(runtime: Rc<Runtime>, file_hover: NativeFileHover) {
    let runtime_clone = runtime.clone();
    let event_closure = Closure::new(
        move |event: web_sys_x::Event, name: String, element_id: u64, bubbles: bool| {
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

    let mounted_closure = Closure::new(
        move |element: web_sys_x::Element, element_id: u64, bubbles: bool| {
            handle_mounted_from_js(&runtime, element, element_id, bubbles)
        },
    );

    setEventHandler(event_closure);
    setMountedHandler(mounted_closure);
}

/// Check if the event name is a drag event.
fn is_drag_event(name: &str) -> bool {
    matches!(
        name,
        "dragenter" | "dragover" | "dragleave" | "drop" | "dragend" | "drag" | "dragstart"
    )
}

/// Handle an event from JavaScript, returning whether to prevent default.
fn handle_event_from_js(
    runtime: &Rc<Runtime>,
    file_hover: &NativeFileHover,
    event: web_sys_x::Event,
    name: String,
    element_id: u64,
    bubbles: bool,
) -> bool {
    use wry_bindgen::JsCast;

    // Get the target element for the event
    let target = event
        .target()
        .and_then(|t| t.dyn_into::<web_sys_x::Element>().ok())
        .unwrap_or_else(|| {
            dioxus_web_sys_events::load_document()
                .document_element()
                .expect("document should have a root element")
        });

    // For drag events, we need to inject native file paths from the file_hover context
    let platform_event: PlatformEventData = if is_drag_event(&name) {
        // Get native file paths from the file_hover
        let native_files = file_hover
            .current()
            .map(|evt| match evt {
                wry::DragDropEvent::Drop { paths, .. } => paths,
                wry::DragDropEvent::Enter { paths, .. } => paths,
                _ => vec![],
            })
            .unwrap_or_default();

        // Create a DesktopFileDragEvent with native file paths
        let drag_event: web_sys_x::DragEvent =
            event.dyn_into().expect("drag event should be DragEvent");
        let desktop_drag = DesktopFileDragEvent::new(Synthetic::new(drag_event), native_files);
        PlatformEventData::new(Box::new(desktop_drag))
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

/// Handle a mounted event from JavaScript.
fn handle_mounted_from_js(
    runtime: &Rc<Runtime>,
    element: web_sys_x::Element,
    element_id: u64,
    bubbles: bool,
) {
    // For mounted events, we pass the element directly as the event data
    let platform_event = PlatformEventData::new(Box::new(element));

    let element = ElementId(element_id as usize);
    let event = dioxus_core::Event::new(Rc::new(platform_event) as Rc<dyn Any>, bubbles);
    runtime.handle_event("mounted", event, element);
}

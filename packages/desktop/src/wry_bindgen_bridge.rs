//! wry-bindgen integration for dioxus desktop.
//!
//! This module provides the bridge between dioxus desktop and wry-bindgen,
//! enabling typed Rust<->JS communication through wry-bindgen's FFI mechanism.
//! The bridge itself lives in `dioxus-web-sys-events` and is shared with the web
//! renderer; this module only adds the desktop-specific event conversion.

use crate::file_upload::{DesktopFileDragEvent, NativeFileHover};
use dioxus_core::Runtime;
use dioxus_html::PlatformEventData;
use dioxus_web_sys_events::{Synthetic, virtual_event_from_websys_event};
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// Initialize the event handler closures for the wry-bindgen bridge.
///
/// This should be called once during VirtualDom initialization on the DOM thread.
pub fn setup_event_handler(runtime: Rc<Runtime>, file_hover: NativeFileHover) {
    dioxus_web_sys_events::set_event_handler(runtime, move |event, target, name| {
        // For drag events, we need to inject native file paths from the file_hover context. A
        // JS-dispatched plain `Event` with a drag name (synthetic events are a supported pattern)
        // fails the `DragEvent` cast and falls back to the generic conversion instead of panicking.
        if dioxus_web_sys_events::is_drag_event(name) {
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
        }
    });
}

//! The Rust side of the JS → Rust event bridge shared by the web and desktop renderers.

use dioxus_core::{ElementId, Runtime};
use dioxus_html::PlatformEventData;
use std::any::Any;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::event_type_matches;

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
    fn setEventHandler(handler: &Closure<dyn FnMut(web_sys::Event, String, u64, bool) -> bool>);
    fn mountedElementById(id: u64) -> Option<web_sys::Element>;
}

/// Install the global `window.rustEventHandler` that the interpreter's `handleEvent` calls
/// with the raw event, the event name, the resolved dioxus element id, and whether the event
/// bubbles. The handler returns whether `preventDefault` should be called.
///
/// `convert` turns the raw event and its target into [`PlatformEventData`]; renderers use it
/// to inject platform-specific event types (e.g. desktop's native file drag events).
pub fn set_event_handler(
    runtime: Rc<Runtime>,
    mut convert: impl FnMut(web_sys::Event, web_sys::Element, &str) -> PlatformEventData + 'static,
) {
    let closure: Closure<dyn FnMut(web_sys::Event, String, u64, bool) -> bool> = Closure::new(
        move |event: web_sys::Event, name: String, element_id: u64, bubbles: bool| {
            handle_event_from_js(
                &runtime,
                event,
                &name,
                ElementId(element_id as usize),
                bubbles,
                &mut convert,
            )
        },
    );

    setEventHandler(&closure);
    closure.forget();
}

/// Handle an event from JavaScript, returning whether to prevent default.
fn handle_event_from_js(
    runtime: &Rc<Runtime>,
    event: web_sys::Event,
    name: &str,
    element: ElementId,
    bubbles: bool,
    convert: &mut impl FnMut(web_sys::Event, web_sys::Element, &str) -> PlatformEventData,
) -> bool {
    // Drop events whose JS type doesn't match what the converters will unchecked-cast to
    // (e.g. a user-dispatched plain `Event` with a typed name like "keydown")
    if !event_type_matches(name, &event) {
        return false;
    }

    // Get the target element for the event
    let target = event
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .unwrap_or_else(|| {
            crate::load_document()
                .document_element()
                .expect("document should have a root element")
        });

    let platform_event = convert(event, target, name);
    dispatch_event_to_runtime(runtime, name, platform_event, element, bubbles)
}

/// Dispatch a converted event into the runtime, returning whether to prevent default.
pub fn dispatch_event_to_runtime(
    runtime: &Rc<Runtime>,
    name: &str,
    data: PlatformEventData,
    element: ElementId,
    bubbles: bool,
) -> bool {
    let event = dioxus_core::Event::new(Rc::new(data) as Rc<dyn Any>, bubbles);
    runtime.handle_event(name, event.clone(), element);

    !event.default_action_enabled()
}

/// Dispatch a mounted event for the element, looking it up in the interpreter's node map.
/// This should only be called after the edit batch that created the listener has been applied
/// to the DOM.
#[cfg(feature = "mounted")]
pub fn handle_mounted_event(runtime: &Rc<Runtime>, element_id: ElementId) {
    let Some(element) = mountedElementById(element_id.0 as u64) else {
        return;
    };

    crate::dispatch_mounted_event(runtime, element_id, element);
}

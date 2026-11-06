use super::{Synthetic, WebEventExt};
use dioxus_html::HasCustomData;
use wasm_bindgen::JsCast;

impl HasCustomData for Synthetic<web_sys::Event> {
    fn detail(&self) -> Option<String> {
        self.event
            .dyn_ref::<web_sys::CustomEvent>()
            .and_then(|event| event.detail().as_string())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl WebEventExt for dioxus_html::CustomData {
    type WebEvent = web_sys::CustomEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::Event>()
            .and_then(|event| event.dyn_ref::<web_sys::CustomEvent>())
            .cloned()
    }
}

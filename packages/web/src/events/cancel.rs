use super::{Synthetic, WebEventExt};
use dioxus_html::HasCancelData;

impl HasCancelData for Synthetic<web_sys::Event> {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl WebEventExt for dioxus_html::CancelData {
    type WebEvent = web_sys::Event;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::Event>().cloned()
    }
}

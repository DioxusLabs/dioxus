use dioxus_html::HasMediaData;

use super::{Synthetic, WebEventExt};

impl HasMediaData for Synthetic<web_sys::Event> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WebEventExt for dioxus_html::MediaData {
    type WebEvent = web_sys::Event;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<Synthetic<web_sys::Event>>()
            .map(|e| e.event.clone())
    }
}

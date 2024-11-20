use dioxus_html::HasFocusData;

use super::{Synthetic, WebEventExt};

impl HasFocusData for Synthetic<web_sys::FocusEvent> {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl WebEventExt for dioxus_html::FocusData {
    type WebEvent = web_sys::FocusEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::FocusEvent>().cloned()
    }
}

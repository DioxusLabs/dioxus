use dioxus_html::HasClipboardData;
use web_sys::ClipboardEvent;

use super::{Synthetic, WebEventExt};

impl HasClipboardData for Synthetic<ClipboardEvent> {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl WebEventExt for dioxus_html::ClipboardData {
    type WebEvent = web_sys::ClipboardEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::ClipboardEvent>().cloned()
    }
}

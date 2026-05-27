use dioxus_html::HasCompositionData;
use web_sys::CompositionEvent;

use super::{Synthetic, WebEventExt};

impl HasCompositionData for Synthetic<CompositionEvent> {
    fn data(&self) -> std::string::String {
        self.event.data().unwrap_or_default()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl WebEventExt for dioxus_html::CompositionData {
    type WebEvent = web_sys::CompositionEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::CompositionEvent>().cloned()
    }
}

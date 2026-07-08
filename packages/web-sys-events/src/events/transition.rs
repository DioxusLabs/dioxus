use dioxus_html::HasTransitionData;
use web_sys::TransitionEvent;

use super::{Synthetic, WebEventExt};

impl HasTransitionData for Synthetic<TransitionEvent> {
    fn elapsed_time(&self) -> f32 {
        self.event.elapsed_time()
    }

    fn property_name(&self) -> String {
        self.event.property_name()
    }

    fn pseudo_element(&self) -> String {
        self.event.pseudo_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl WebEventExt for dioxus_html::TransitionData {
    type WebEvent = web_sys::TransitionEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::TransitionEvent> {
        self.downcast::<web_sys::TransitionEvent>().cloned()
    }
}

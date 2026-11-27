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
}

impl WebEventExt for dioxus_html::TransitionData {
    type WebEvent = TransitionEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<Synthetic<TransitionEvent>>()
            .map(|backing| backing.event.clone())
    }
}

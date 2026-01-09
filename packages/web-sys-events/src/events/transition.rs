use dioxus_html::HasTransitionData;
use web_sys::TransitionEvent;

use super::Synthetic;

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

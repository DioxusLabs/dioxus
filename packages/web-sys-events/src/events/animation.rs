use dioxus_html::HasAnimationData;
use web_sys_x::AnimationEvent;

use super::{Synthetic, WebEventExt};

impl HasAnimationData for Synthetic<AnimationEvent> {
    fn animation_name(&self) -> String {
        self.event.animation_name()
    }

    fn pseudo_element(&self) -> String {
        self.event.pseudo_element()
    }

    fn elapsed_time(&self) -> f32 {
        self.event.elapsed_time()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl WebEventExt for dioxus_html::AnimationData {
    type WebEvent = web_sys_x::AnimationEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys_x::AnimationEvent> {
        self.downcast::<web_sys_x::AnimationEvent>().cloned()
    }
}

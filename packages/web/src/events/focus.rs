use dioxus_html::HasFocusData;

use super::{Synthetic, WebEventExt};

impl HasFocusData for Synthetic<web_sys::FocusEvent> {}

impl WebEventExt for dioxus_html::FocusData {
    type WebEvent = web_sys::FocusEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<Synthetic<web_sys::FocusEvent>>()
            .map(|backing| backing.event.clone())
    }
}

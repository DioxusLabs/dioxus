use std::any::Any;

use dioxus_html::HasImageData;
use web_sys::Event;

use super::WebEventExt;

#[derive(Clone)]
pub(crate) struct WebImageEvent {
    raw: Event,
    error: bool,
}

impl WebImageEvent {
    pub fn new(raw: Event, error: bool) -> Self {
        Self { raw, error }
    }
}

impl HasImageData for WebImageEvent {
    fn load_error(&self) -> bool {
        self.error
    }

    fn as_any(&self) -> &dyn Any {
        &self.raw
    }
}

impl WebEventExt for dioxus_html::ImageData {
    type WebEvent = Event;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Event> {
        self.downcast::<WebImageEvent>().map(|e| e.raw.clone())
    }
}

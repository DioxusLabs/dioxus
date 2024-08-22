use std::any::Any;

use dioxus_html::HasImageData;
use web_sys::Event;

#[derive(Clone)]
pub struct WebImageEvent {
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
        &self.raw as &dyn Any
    }
}

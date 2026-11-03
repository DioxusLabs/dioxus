use dioxus_html::{HasBeforeInputData, InputType};
use web_sys::{Element, InputEvent};

use super::WebEventExt;

pub(crate) struct WebBeforeInputData {
    element: Element,
    event: InputEvent,
}

impl WebBeforeInputData {
    pub fn new(element: Element, event: InputEvent) -> Self {
        Self { element, event }
    }
}

impl HasBeforeInputData for WebBeforeInputData {
    fn input_type(&self) -> InputType {
        InputType::from(self.event.input_type().as_str())
    }

    fn data(&self) -> Option<String> {
        self.event.data()
    }

    fn is_composing(&self) -> bool {
        self.event.is_composing()
    }

    fn value(&self) -> String {
        super::editable_element_value(&self.element).unwrap_or_default()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.event as &dyn std::any::Any
    }
}

impl WebEventExt for dioxus_html::BeforeInputData {
    type WebEvent = web_sys::InputEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::InputEvent>().cloned()
    }
}

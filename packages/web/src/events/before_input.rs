use dioxus_html::HasBeforeInputData;
use wasm_bindgen::JsCast;
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
    fn input_type(&self) -> String {
        self.event.input_type()
    }

    fn data(&self) -> Option<String> {
        self.event.data()
    }

    fn is_composing(&self) -> bool {
        self.event.is_composing()
    }

    fn value(&self) -> String {
        let target = &self.element;
        target
            .dyn_ref()
            .map(
                |input: &web_sys::HtmlInputElement| match input.type_().as_str() {
                    "checkbox" => match input.checked() {
                        true => "true".to_string(),
                        false => "false".to_string(),
                    },
                    _ => input.value(),
                },
            )
            .or_else(|| {
                target
                    .dyn_ref()
                    .map(|input: &web_sys::HtmlTextAreaElement| input.value())
            })
            .or_else(|| {
                target
                    .dyn_ref()
                    .map(|input: &web_sys::HtmlSelectElement| input.value())
            })
            .or_else(|| {
                target
                    .dyn_ref::<web_sys::HtmlElement>()
                    .and_then(|el| el.text_content())
            })
            .unwrap_or_default()
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

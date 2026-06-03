use super::{Synthetic, WebEventExt};
use dioxus_html::{HasSelectionData, SelectionDirection, TextSelection};
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement, HtmlTextAreaElement};

impl HasSelectionData for Synthetic<Event> {
    fn selection(&self) -> Option<TextSelection> {
        with_text_control(&self.event, |input| {
            let start = input.selection_start().ok().flatten()? as usize;
            let end = input.selection_end().ok().flatten()? as usize;
            let direction = input
                .selection_direction()
                .ok()
                .flatten()
                .as_deref()
                .map(selection_direction_from_web)
                .unwrap_or_default();

            Some(TextSelection::new(start..end, direction))
        })
        .flatten()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

fn selection_direction_from_web(direction: &str) -> SelectionDirection {
    match direction {
        "forward" => SelectionDirection::Forward,
        "backward" => SelectionDirection::Backward,
        _ => SelectionDirection::None,
    }
}

impl WebEventExt for dioxus_html::SelectionData {
    type WebEvent = web_sys::Event;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::Event>().cloned()
    }
}

fn with_text_control<T>(event: &Event, f: impl FnOnce(TextControl<'_>) -> T) -> Option<T> {
    event.target().and_then(|target| {
        if let Some(input) = target.dyn_ref::<HtmlInputElement>() {
            Some(f(TextControl::Input(input)))
        } else {
            target
                .dyn_ref::<HtmlTextAreaElement>()
                .map(|textarea| f(TextControl::TextArea(textarea)))
        }
    })
}

enum TextControl<'a> {
    Input(&'a HtmlInputElement),
    TextArea(&'a HtmlTextAreaElement),
}

impl TextControl<'_> {
    fn selection_start(&self) -> Result<Option<u32>, wasm_bindgen::JsValue> {
        match self {
            Self::Input(input) => input.selection_start(),
            Self::TextArea(textarea) => textarea.selection_start(),
        }
    }

    fn selection_end(&self) -> Result<Option<u32>, wasm_bindgen::JsValue> {
        match self {
            Self::Input(input) => input.selection_end(),
            Self::TextArea(textarea) => textarea.selection_end(),
        }
    }

    fn selection_direction(&self) -> Result<Option<String>, wasm_bindgen::JsValue> {
        match self {
            Self::Input(input) => input.selection_direction(),
            Self::TextArea(textarea) => textarea.selection_direction(),
        }
    }
}

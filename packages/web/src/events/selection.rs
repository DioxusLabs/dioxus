use super::{Synthetic, WebEventExt};
use dioxus_html::HasSelectionData;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement, HtmlTextAreaElement};

impl HasSelectionData for Synthetic<Event> {
    fn selection_start(&self) -> Option<usize> {
        with_text_control(&self.event, |input| {
            input
                .selection_start()
                .ok()
                .flatten()
                .map(|value| value as usize)
        })
        .flatten()
    }

    fn selection_end(&self) -> Option<usize> {
        with_text_control(&self.event, |input| {
            input
                .selection_end()
                .ok()
                .flatten()
                .map(|value| value as usize)
        })
        .flatten()
    }

    fn selection_direction(&self) -> Option<String> {
        with_text_control(&self.event, |input| {
            input.selection_direction().ok().flatten()
        })
        .flatten()
    }

    fn selected_text(&self) -> String {
        if let Some(text) = with_text_control(&self.event, selected_text_in_control) {
            return text;
        }

        web_sys::window()
            .and_then(|window| window.get_selection().ok().flatten())
            .map(|selection| selection.to_string().as_string().unwrap_or_default())
            .unwrap_or_default()
    }

    fn anchor_offset(&self) -> Option<usize> {
        web_sys::window()
            .and_then(|window| window.get_selection().ok().flatten())
            .map(|selection| selection.anchor_offset() as usize)
    }

    fn focus_offset(&self) -> Option<usize> {
        web_sys::window()
            .and_then(|window| window.get_selection().ok().flatten())
            .map(|selection| selection.focus_offset() as usize)
    }

    fn is_collapsed(&self) -> Option<bool> {
        web_sys::window()
            .and_then(|window| window.get_selection().ok().flatten())
            .map(|selection| selection.is_collapsed())
    }

    fn range_count(&self) -> Option<usize> {
        web_sys::window()
            .and_then(|window| window.get_selection().ok().flatten())
            .map(|selection| selection.range_count() as usize)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
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

    fn value(&self) -> String {
        match self {
            Self::Input(input) => input.value(),
            Self::TextArea(textarea) => textarea.value(),
        }
    }
}

fn selected_text_in_control(control: TextControl<'_>) -> String {
    let start = control
        .selection_start()
        .ok()
        .flatten()
        .map(|value| value as usize)
        .unwrap_or_default();
    let end = control
        .selection_end()
        .ok()
        .flatten()
        .map(|value| value as usize)
        .unwrap_or(start);
    let value = control.value();
    let start = byte_index_for_utf16(&value, start);
    let end = byte_index_for_utf16(&value, end);

    value[start.min(end)..end.max(start)].to_string()
}

fn byte_index_for_utf16(value: &str, utf16_offset: usize) -> usize {
    let mut current = 0;
    for (byte_index, c) in value.char_indices() {
        if current >= utf16_offset {
            return byte_index;
        }

        let next = current + c.len_utf16();
        if next > utf16_offset {
            return byte_index;
        }

        current = next;
    }

    value.len()
}

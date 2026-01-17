use super::WebEventExt;
use crate::WebFileData;
use dioxus_html::{FileData, FormValue, HasFileData, HasFormData};
use js_sys_x::Array;
use std::any::Any;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast};
use web_sys_x::{Element, Event, FileReader};

/// Web-sys form data implementation.
/// Wraps a web_sys_x Element and Event to provide form data extraction.
pub struct WebFormData {
    /// The target element of the form event
    pub element: Element,
    /// The raw form event
    pub event: Event,
}

impl WebEventExt for dioxus_html::FormData {
    type WebEvent = Event;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<Event>().cloned()
    }
}

impl WebFormData {
    pub fn new(element: Element, event: Event) -> Self {
        Self { element, event }
    }
}

impl HasFormData for WebFormData {
    fn value(&self) -> String {
        let target = &self.element;
        target
            .dyn_ref()
            .map(|input: &web_sys_x::HtmlInputElement| {
                // todo: special case more input types
                match input.type_().as_str() {
                    "checkbox" => {
                        match input.checked() {
                            true => "true".to_string(),
                            false => "false".to_string(),
                        }
                    },
                    _ => {
                        input.value()
                    }
                }
            })
            .or_else(|| {
                target
                    .dyn_ref()
                    .map(|input: &web_sys_x::HtmlTextAreaElement| input.value())
            })
            // select elements are NOT input events - because - why woudn't they be??
            .or_else(|| {
                target
                    .dyn_ref()
                    .map(|input: &web_sys_x::HtmlSelectElement| input.value())
            })
            .or_else(|| {
                target
                    .dyn_ref::<web_sys_x::HtmlElement>()
                    .unwrap()
                    .text_content()
            })
            .expect("only an InputElement or TextAreaElement or an element with contenteditable=true can have an oninput event listener")
    }

    fn values(&self) -> Vec<(String, FormValue)> {
        let mut values = Vec::new();

        // Try to find the form element - either the element itself is a form,
        // or we need to find the closest ancestor form
        let form_element = self
            .element
            .dyn_ref::<web_sys_x::HtmlFormElement>()
            .cloned()
            .or_else(|| {
                self.element
                    .closest("form")
                    .ok()
                    .flatten()
                    .and_then(|el| el.dyn_into::<web_sys_x::HtmlFormElement>().ok())
            });

        // try to fill in form values
        if let Some(form) = form_element {
            let form_data = web_sys_x::FormData::new_with_form(&form).unwrap();

            for entry in form_data.entries().into_iter().flatten() {
                if let Ok(array) = entry.dyn_into::<Array>() {
                    if let Some(name) = array.get(0).as_string() {
                        let value = array.get(1);
                        if let Some(file) = value.dyn_ref::<web_sys_x::File>() {
                            if file.name().is_empty() {
                                values.push((name, FormValue::File(None)));
                            } else {
                                let data =
                                    WebFileData::new(file.clone(), FileReader::new().unwrap());
                                let as_file = FileData::new(data);

                                values.push((name, FormValue::File(Some(as_file))));
                            }
                        } else if let Some(s) = value.as_string() {
                            values.push((name, FormValue::Text(s)));
                        }
                    }
                }
            }
        } else if let Some(select) = self.element.dyn_ref::<web_sys_x::HtmlSelectElement>() {
            // try to fill in select element values
            let options = get_select_data(select);
            for option in &options {
                values.push((select.name(), FormValue::Text(option.clone())));
            }
        }

        values
    }

    fn as_any(&self) -> &dyn Any {
        &self.event as &dyn Any
    }

    fn valid(&self) -> bool {
        self.event
            .target()
            .and_then(|t| t.dyn_into::<web_sys_x::HtmlInputElement>().ok())
            .map(|input| input.check_validity())
            .unwrap_or(true)
    }
}

impl HasFileData for WebFormData {
    fn files(&self) -> Vec<FileData> {
        use wasm_bindgen::JsCast;
        self.event
            .target()
            .and_then(|t| t.dyn_into::<web_sys_x::HtmlInputElement>().ok())
            .and_then(|input| input.files())
            .map(crate::files::WebFileEngine::new)
            .map(|engine| engine.to_files())
            .unwrap_or_default()
    }
}

// web-sys does not expose the keys api for select data, so we need to manually bind to it
#[wasm_bindgen(inline_js = r#"
export function get_select_data(select) {
    let values = [];
    for (let i = 0; i < select.options.length; i++) {
      let option = select.options[i];
      if (option.selected) {
        values.push(option.value.toString());
      }
    }

    return values;
}
"#)]
extern "C" {
    fn get_select_data(select: &web_sys_x::HtmlSelectElement) -> Vec<String>;
}

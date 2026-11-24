use super::WebEventExt;
use crate::WebFileData;
use dioxus_html::{FileData, FormValue, HasFileData, HasFormData};
use js_sys::Array;
use std::any::Any;
use wasm_bindgen::{JsCast, prelude::wasm_bindgen};
use web_sys::{Element, Event, FileReader};

pub(crate) struct WebFormData {
    element: Element,
    event: Event,
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
        super::editable_element_value(&self.element)
            .expect("only an InputElement or TextAreaElement or an element with contenteditable=true can have an oninput event listener")
    }

    fn values(&self) -> Vec<(String, FormValue)> {
        let mut values = Vec::new();

        // try to fill in form values
        if let Some(form) = self.element.dyn_ref::<web_sys::HtmlFormElement>() {
            // Passing the submitter includes the name/value of the button that submitted the form
            let submitter = self
                .event
                .dyn_ref::<web_sys::SubmitEvent>()
                .and_then(|event| event.submitter());
            let form_data = form_data_with_submitter(form, submitter.as_ref());

            for entry in form_data.entries().into_iter().flatten() {
                if let Ok(array) = entry.dyn_into::<Array>()
                    && let Some(name) = array.get(0).as_string()
                {
                    let value = array.get(1);
                    if let Some(file) = value.dyn_ref::<web_sys::File>() {
                        if file.name().is_empty() {
                            values.push((name, FormValue::File(None)));
                        } else {
                            let data = WebFileData::new(file.clone(), FileReader::new().unwrap());
                            let as_file = FileData::new(data);

                            values.push((name, FormValue::File(Some(as_file))));
                        }
                    } else if let Some(s) = value.as_string() {
                        values.push((name, FormValue::Text(s)));
                    }
                }
            }
        } else if let Some(select) = self.element.dyn_ref::<web_sys::HtmlSelectElement>() {
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
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            .map(|input| input.check_validity())
            .unwrap_or(true)
    }
}

impl HasFileData for WebFormData {
    fn files(&self) -> Vec<FileData> {
        use wasm_bindgen::JsCast;
        self.event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            .and_then(|input| input.files())
            .map(crate::files::WebFileEngine::new)
            .map(|engine| engine.to_files())
            .unwrap_or_default()
    }
}

// web-sys does not expose the keys api for select data or the submitter argument of the FormData
// constructor, so we need to manually bind to them
#[wasm_bindgen(inline_js = r#"
export function form_data_with_submitter(form, submitter) {
    return new FormData(form, submitter);
}

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
    fn form_data_with_submitter(
        form: &web_sys::HtmlFormElement,
        submitter: Option<&web_sys::HtmlElement>,
    ) -> web_sys::FormData;

    fn get_select_data(select: &web_sys::HtmlSelectElement) -> Vec<String>;
}

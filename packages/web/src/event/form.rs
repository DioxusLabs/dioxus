use std::{any::Any, collections::HashMap};

use dioxus_html::{FormValue, HasFileData, HasFormData};
use js_sys::Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use web_sys::{Element, Event};

pub struct WebFormData {
    pub element: Element,
    pub raw: Event,
}

impl WebFormData {
    pub fn new(element: Element, raw: Event) -> Self {
        Self { element, raw }
    }
}

impl HasFormData for WebFormData {
    fn value(&self) -> String {
        let target = &self.element;
        target
        .dyn_ref()
        .map(|input: &web_sys::HtmlInputElement| {
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
                .map(|input: &web_sys::HtmlTextAreaElement| input.value())
        })
        // select elements are NOT input events - because - why woudn't they be??
        .or_else(|| {
            target
                .dyn_ref()
                .map(|input: &web_sys::HtmlSelectElement| input.value())
        })
        .or_else(|| {
            target
                .dyn_ref::<web_sys::HtmlElement>()
                .unwrap()
                .text_content()
        })
        .expect("only an InputElement or TextAreaElement or an element with contenteditable=true can have an oninput event listener")
    }

    fn values(&self) -> HashMap<String, FormValue> {
        let mut values = HashMap::new();

        fn insert_value(map: &mut HashMap<String, FormValue>, key: String, new_value: String) {
            map.entry(key.clone()).or_default().0.push(new_value);
        }

        // try to fill in form values
        if let Some(form) = self.element.dyn_ref::<web_sys::HtmlFormElement>() {
            let form_data = get_form_data(form);
            for value in form_data.entries().into_iter().flatten() {
                if let Ok(array) = value.dyn_into::<Array>() {
                    if let Some(name) = array.get(0).as_string() {
                        if let Ok(item_values) = array.get(1).dyn_into::<Array>() {
                            item_values
                                .iter()
                                .filter_map(|v| v.as_string())
                                .for_each(|v| insert_value(&mut values, name.clone(), v));
                        } else if let Ok(item_value) = array.get(1).dyn_into::<JsValue>() {
                            insert_value(&mut values, name, item_value.as_string().unwrap());
                        }
                    }
                }
            }
        } else if let Some(select) = self.element.dyn_ref::<web_sys::HtmlSelectElement>() {
            // try to fill in select element values
            let options = get_select_data(select);
            values.insert("options".to_string(), FormValue(options));
        }

        values
    }

    fn as_any(&self) -> &dyn Any {
        &self.raw as &dyn Any
    }
}

impl HasFileData for WebFormData {
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn dioxus_html::FileEngine>> {
        use super::file::WebFileEngine;

        let files = self
            .element
            .dyn_ref()
            .and_then(|input: &web_sys::HtmlInputElement| {
                input.files().and_then(|files| {
                    #[allow(clippy::arc_with_non_send_sync)]
                    WebFileEngine::new(files).map(|f| {
                        std::sync::Arc::new(f) as std::sync::Arc<dyn dioxus_html::FileEngine>
                    })
                })
            });

        files
    }
}

// web-sys does not expose the keys api for form data, so we need to manually bind to it
#[wasm_bindgen(inline_js = r#"
export function get_form_data(form) {
    let values = new Map();
    const formData = new FormData(form);

    for (let name of formData.keys()) {
        values.set(name, formData.getAll(name));
    }

    return values;
}
"#)]
extern "C" {
    fn get_form_data(form: &web_sys::HtmlFormElement) -> js_sys::Map;
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
    fn get_select_data(select: &web_sys::HtmlSelectElement) -> Vec<String>;
}

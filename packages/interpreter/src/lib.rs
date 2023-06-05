const INTERPRETER_JS: &str = include_str!("./js/interpreter.js");
const SET_ATTRIBUTE_INNER_JS: &str = include_str!("./js/setAttributeInner.js");
const JS_STRS: &[&str] = &[INTERPRETER_JS, SET_ATTRIBUTE_INNER_JS];

/// Get all JS files in a single string with exports and imports stripped.
pub fn js_as_single_string() -> String {
    let mut value = String::new();

    for js in JS_STRS.iter() {
        value.push_str(js);
        value.push('\n');
    }

    // Remove exports & imports
    let value = value.replace("export", "");
    let mut final_value = String::new();
    for line in value.lines() {
        if line.contains("import") {
            continue;
        }
        final_value.push_str(line);
        final_value.push('\n');
    }

    final_value
}

/// Get the setAttributeInner.js file, removing exports.
pub fn js_set_attribute_inner() -> String {
    SET_ATTRIBUTE_INNER_JS.replace("export", "")
}

#[cfg(feature = "sledgehammer")]
mod sledgehammer_bindings;
#[cfg(feature = "sledgehammer")]
pub use sledgehammer_bindings::*;

/// Common bindings for minimal usage.
#[cfg(feature = "minimal_bindings")]
pub mod minimal_bindings {
    use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
    #[wasm_bindgen(module = "/src/js/setAttributeInner.js")]
    extern "C" {
        pub fn setAttributeInner(node: JsValue, name: &str, value: JsValue, ns: Option<&str>);
    }
}

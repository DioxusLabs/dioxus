//! Utilities specific to websys

use dioxus_core::*;

/// Get a closure that executes any JavaScript in the webpage.
///
/// # Safety
///
/// Please be very careful with this function. A script with too many dynamic
/// parts is practically asking for a hacker to find an XSS vulnerability in
/// it. **This applies especially to web targets, where the JavaScript context
/// has access to most, if not all of your application data.**
///
/// # Panics
///
/// The closure will panic if the provided script is not valid JavaScript code
/// or if it returns an uncaught error.
pub fn use_eval<S: std::string::ToString>(_cx: &ScopeState) -> impl Fn(S) {
    |script| {
        js_sys::Function::new_no_args(&script.to_string())
            .call0(&wasm_bindgen::JsValue::NULL)
            .expect("failed to eval script");
    }
}

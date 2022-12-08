//! Utilities specific to websys

use std::str::FromStr;

use dioxus_core::*;
use serde::de::Error;
use serde_json::Value;

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
pub fn use_eval<S: std::string::ToString>(
    cx: &ScopeState,
) -> &dyn Fn(S) -> Result<Value, serde_json::Error> {
    cx.use_hook(|| {
        |script: S| {
            let body = script.to_string();
            if let Ok(value) =
                js_sys::Function::new_no_args(&body).call0(&wasm_bindgen::JsValue::NULL)
            {
                if let Ok(stringified) = js_sys::JSON::stringify(&value) {
                    let string: String = stringified.into();
                    Value::from_str(&string)
                } else {
                    Err(serde_json::Error::custom("Failed to stringify result"))
                }
            } else {
                Err(serde_json::Error::custom("Failed to execute script"))
            }
        }
    })
}

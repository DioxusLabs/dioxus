//! Utilities specific to websys

use std::{
    future::{IntoFuture, Ready},
    str::FromStr,
};

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
pub fn use_eval<S: std::string::ToString>(cx: &ScopeState) -> &dyn Fn(S) -> EvalResult {
    cx.use_hook(|| {
        |script: S| {
            let body = script.to_string();
            EvalResult {
                value: if let Ok(value) =
                    js_sys::Function::new_no_args(&body).call0(&wasm_bindgen::JsValue::NULL)
                {
                    if let Ok(stringified) = js_sys::JSON::stringify(&value) {
                        if !stringified.is_undefined() && stringified.is_valid_utf16() {
                            let string: String = stringified.into();
                            Value::from_str(&string)
                        } else {
                            Err(serde_json::Error::custom("Failed to stringify result"))
                        }
                    } else {
                        Err(serde_json::Error::custom("Failed to stringify result"))
                    }
                } else {
                    Err(serde_json::Error::custom("Failed to execute script"))
                },
            }
        }
    })
}

/// A wrapper around the result of a JavaScript evaluation.
/// This implements IntoFuture to be compatible with the desktop renderer's EvalResult.
pub struct EvalResult {
    value: Result<Value, serde_json::Error>,
}

impl EvalResult {
    /// Get the result of the Javascript execution.
    pub fn get(self) -> Result<Value, serde_json::Error> {
        self.value
    }
}

impl IntoFuture for EvalResult {
    type Output = Result<Value, serde_json::Error>;

    type IntoFuture = Ready<Result<Value, serde_json::Error>>;

    fn into_future(self) -> Self::IntoFuture {
        std::future::ready(self.value)
    }
}

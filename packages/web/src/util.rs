//! Utilities specific to websys

use std::{cell::RefCell, rc::Rc};

use dioxus_core::*;
use js_sys::Function;
use wasm_bindgen::prelude::*;

/// Get a closure that executes any JavaScript in the webpage.
///
/// # Safety
///
/// Please be very careful with this function. A script with too many dynamic
/// parts is practically asking for a hacker to find an XSS vulnerability in
/// it. **This applies especially to web targets, where the JavaScript context
/// has access to most, if not all of your application data.**
pub fn use_eval<S: ToString>(cx: &ScopeState) -> &dyn Fn(S) -> UseEval {
    let eval = |script: S| {
        let js = script.to_string();
        UseEval::new(js)
    };

    cx.use_hook(|| eval)
}

/// UseEval
pub struct UseEval {
    dioxus: Dioxus,
    received: Rc<RefCell<Vec<JsValue>>>,
}

impl UseEval {
    /// Create a new UseEval with the specified JS
    pub fn new(js: String) -> Self {
        let received = Rc::new(RefCell::new(Vec::new()));
        let received2 = received.clone();

        let a = Closure::<dyn FnMut(JsValue)>::new(move |data| {
           received2.borrow_mut().push(data);
        });

        let dioxus = Dioxus::new(a.as_ref().unchecked_ref());
        a.forget();

        Function::new_with_args("dioxus", &js).call1(&JsValue::NULL, &dioxus).unwrap();

        Self {
            dioxus,
            received,
        }
    }

    /// Send a message to the evaluated JS code
    pub fn send(&self, data: JsValue) {
        self.dioxus.rustSend(data);
    }

    /// Receives a message from the evaluated JS code. Last in, first out.
    pub fn recv(&self) -> JsValue {
        loop {
            if let Some(data) = self.received.as_ref().clone().into_inner().pop() {
                return data;
            }
        }
    }
}

#[wasm_bindgen(module = "/src/eval.js")]
extern "C" {
    pub type Dioxus;

    #[wasm_bindgen(constructor)]
    pub fn new(recv_callback: &Function) -> Dioxus;

    #[wasm_bindgen(method)]
    pub fn rustSend(this: &Dioxus, data: JsValue);

}

/*pub fn use_eval<S: std::string::ToString>(cx: &ScopeState) -> &dyn Fn(S) -> EvalResult {
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
}*/

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

const PROMISE_WRAPPER: &str = r#"
    return new Promise(async (resolve, _reject) => {
        {JS_CODE}
        resolve(null);
    });
    "#;

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

        // Wrap the evaluated JS in a promise so that wasm can continue running (send/receive data from js)
        let code = PROMISE_WRAPPER.replace("{JS_CODE}", &js);
        Function::new_with_args("dioxus", &code).call1(&JsValue::NULL, &dioxus).unwrap();

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

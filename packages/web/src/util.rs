//! Utilities specific to websys

use std::{
    cell::RefCell,
    collections::VecDeque,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use dioxus_core::*;
use futures_util::{Stream, StreamExt};
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
pub fn use_eval<S: ToString>(cx: &ScopeState, code: S) -> &mut UseEval {
    let js = code.to_string();
    let eval = UseEval::new(js);
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
    received: Rc<RefCell<MessageQueue>>,
    code: String,
    ran: bool,
}

impl UseEval {
    /// Create a new UseEval with the specified JS
    pub fn new(js: String) -> Self {
        let received = Rc::new(RefCell::new(MessageQueue::new()));
        let received2 = received.clone();

        let a = Closure::<dyn FnMut(JsValue)>::new(move |data| {
            received2.borrow_mut().push(data);
        });

        let dioxus = Dioxus::new(a.as_ref().unchecked_ref());
        a.forget();

        // Wrap the evaluated JS in a promise so that wasm can continue running (send/receive data from js)
        let code = PROMISE_WRAPPER.replace("{JS_CODE}", &js);

        Self {
            dioxus,
            received,
            code,
            ran: false,
        }
    }

    /// Run the evaluated JS code
    /// # Panics
    /// Will panic if the JS code is invalid.
    pub fn run(&mut self) {
        Function::new_with_args("dioxus", &self.code)
            .call1(&JsValue::NULL, &self.dioxus)
            .unwrap();
        self.ran = true;
    }

    /// Send a message to the evaluated JS code
    pub fn send(&self, data: JsValue) {
        self.dioxus.rustSend(data);
    }
    /// Receives a message from the evaluated JS code. First in, first out.
    /// This can't be used at the same time as ``recv`` or messages may be lost.
    pub fn recv_sync(&self) -> JsValue {
        if !self.ran {
            panic!("Js code has not been ran. Please call UseEval.run() before using UseEval.recv_sync()");
        }

        loop {
            if let Some(data) = self.received.as_ref().clone().borrow_mut().pop() {
                return data;
            }
        }
    }
    /// Waits for a new message and returns it. First in, first out.
    /// This can't be used at the same time as ``recv_sync` or messages may be lost.
    pub async fn recv(&self) -> JsValue {
        let mut queue = self.received.as_ref().clone().borrow_mut();
        queue.next().await.expect("stream should never return None")
    }
}

pub struct MessageQueue {
    queue: VecDeque<JsValue>,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Pops an item off the front
    pub fn pop(&mut self) -> Option<JsValue> {
        self.queue.pop_front()
    }

    /// Pushes an item onto the back
    pub fn push(&mut self, value: JsValue) {
        self.queue.push_back(value);
    }
}

impl Stream for MessageQueue {
    type Item = JsValue;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(value) = self.pop() {
            Poll::Ready(Some(value))
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
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

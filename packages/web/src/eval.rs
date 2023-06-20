use async_trait::async_trait;
use dioxus_core::ScopeState;
use dioxus_html::prelude::{EvalError, EvalProvider, Evaluator, MessageQueue};
use futures_util::StreamExt;
use js_sys::Function;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;

pub fn init_eval(cx: &ScopeState) {
    let provider: Rc<dyn EvalProvider> = Rc::new(WebEvalProvider {});
    cx.provide_context(provider);
}

pub struct WebEvalProvider;
impl EvalProvider for WebEvalProvider {
    fn new_evaluator(&self, js: String) -> Box<dyn Evaluator> {
        Box::new(WebEvaluator::new(js))
    }
}

const PROMISE_WRAPPER: &str = r#"
    return new Promise(async (resolve, _reject) => {
        {JS_CODE}
        resolve(null);
    });
    "#;

pub struct WebEvaluator {
    dioxus: Dioxus,
    received: Rc<RefCell<MessageQueue>>,
    code: String,
    ran: bool,
}

impl WebEvaluator {
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
}

#[async_trait(?Send)]
impl Evaluator for WebEvaluator {
    fn run(&mut self) -> Result<(), EvalError> {
        if let Err(e) =
            Function::new_with_args("dioxus", &self.code).call1(&JsValue::NULL, &self.dioxus)
        {
            return Err(EvalError::InvalidJs(
                e.as_string().unwrap_or("unknown".to_string()),
            ));
        }

        self.ran = true;
        Ok(())
    }

    fn send(&self, data: JsValue) -> Result<(), EvalError> {
        if !self.ran {
            return Err(EvalError::NotRan);
        }
        self.dioxus.rustSend(data);
        Ok(())
    }

    async fn recv(&self) -> Result<JsValue, EvalError> {
        if !self.ran {
            return Err(EvalError::NotRan);
        }
        let mut queue = self.received.as_ref().clone().borrow_mut();
        Ok(queue.next().await.expect("stream should never return None"))
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
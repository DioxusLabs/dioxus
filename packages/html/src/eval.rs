#![allow(clippy::await_holding_refcell_ref)]

use async_trait::async_trait;
use dioxus_core::ScopeState;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::rc::Rc;

/// A struct that implements EvalProvider is sent through [`ScopeState`]'s provide_context function
/// so that [`use_eval`] can provide a platform agnostic interface for evaluating JavaScript code.
pub trait EvalProvider {
    fn new_evaluator(&self, js: String) -> Result<Rc<dyn Evaluator>, EvalError>;
}

/// The platform's evaluator.
#[async_trait(?Send)]
pub trait Evaluator {
    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError>;
    /// Receive any queued messages from the evaluated JavaScript.
    async fn recv(&self) -> Result<serde_json::Value, EvalError>;
    /// Gets the return value of the JavaScript
    async fn join(&self) -> Result<serde_json::Value, EvalError>;
}

type EvalCreator = Rc<dyn Fn(&str) -> Result<UseEval, EvalError>>;

/// Get a struct that can execute any JavaScript.
///
/// # Safety
///
/// Please be very careful with this function. A script with too many dynamic
/// parts is practically asking for a hacker to find an XSS vulnerability in
/// it. **This applies especially to web targets, where the JavaScript context
/// has access to most, if not all of your application data.**
#[must_use]
pub fn use_eval(cx: &ScopeState) -> &EvalCreator {
    &*cx.use_hook(|| {
        let eval_provider = cx
            .consume_context::<Rc<dyn EvalProvider>>()
            .expect("evaluator not provided");

        Rc::new(move |script: &str| {
            eval_provider
                .new_evaluator(script.to_string())
                .map(UseEval::new)
        }) as Rc<dyn Fn(&str) -> Result<UseEval, EvalError>>
    })
}

/// A wrapper around the target platform's evaluator.
#[derive(Clone)]
pub struct UseEval {
    evaluator: Rc<dyn Evaluator + 'static>,
}

impl UseEval {
    /// Creates a new UseEval
    pub fn new(evaluator: Rc<dyn Evaluator + 'static>) -> Self {
        Self { evaluator }
    }

    /// Sends a [`serde_json::Value`] to the evaluated JavaScript.
    pub fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        self.evaluator.send(data)
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    pub async fn recv(&self) -> Result<serde_json::Value, EvalError> {
        self.evaluator.recv().await
    }

    /// Gets the return value of the evaluated JavaScript.
    pub async fn join(self) -> Result<serde_json::Value, EvalError> {
        self.evaluator.join().await
    }
}

impl IntoFuture for UseEval {
    type Output = Result<serde_json::Value, EvalError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.join())
    }
}

/// Represents an error when evaluating JavaScript
#[derive(Debug)]
pub enum EvalError {
    /// The provided JavaScript has already been ran.
    Finished,
    /// The provided JavaScript is not valid and can't be ran.
    InvalidJs(String),
    /// Represents an error communicating between JavaScript and Rust.
    Communication(String),
}

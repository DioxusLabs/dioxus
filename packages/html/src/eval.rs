#![allow(clippy::await_holding_refcell_ref)]

use dioxus_core::prelude::*;
use generational_box::GenerationalBox;
use std::future::{poll_fn, Future, IntoFuture};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// A struct that implements EvalProvider is sent through [`ScopeState`]'s provide_context function
/// so that [`use_eval`] can provide a platform agnostic interface for evaluating JavaScript code.
pub trait EvalProvider {
    fn new_evaluator(&self, js: String) -> Result<GenerationalBox<Box<dyn Evaluator>>, EvalError>;
}

/// The platform's evaluator.
pub trait Evaluator {
    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError>;
    /// Receive any queued messages from the evaluated JavaScript.
    fn poll_recv(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, EvalError>>;
    /// Gets the return value of the JavaScript
    fn poll_join(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, EvalError>>;
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
pub fn eval_provider() -> EvalCreator {
    let eval_provider = consume_context::<Rc<dyn EvalProvider>>();

    Rc::new(move |script: &str| {
        eval_provider
            .new_evaluator(script.to_string())
            .map(UseEval::new)
    }) as Rc<dyn Fn(&str) -> Result<UseEval, EvalError>>
}

pub fn eval(script: &str) -> Result<UseEval, EvalError> {
    let eval_provider = dioxus_core::prelude::consume_context::<Rc<dyn EvalProvider>>();

    eval_provider
        .new_evaluator(script.to_string())
        .map(UseEval::new)
}

/// A wrapper around the target platform's evaluator.
#[derive(Clone, Copy)]
pub struct UseEval {
    evaluator: GenerationalBox<Box<dyn Evaluator>>,
}

impl UseEval {
    /// Creates a new UseEval
    pub fn new(evaluator: GenerationalBox<Box<dyn Evaluator + 'static>>) -> Self {
        Self { evaluator }
    }

    /// Sends a [`serde_json::Value`] to the evaluated JavaScript.
    pub fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        self.evaluator.read().send(data)
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    pub async fn recv(&mut self) -> Result<serde_json::Value, EvalError> {
        poll_fn(|cx| match self.evaluator.try_write() {
            Ok(mut evaluator) => evaluator.poll_recv(cx),
            Err(_) => Poll::Ready(Err(EvalError::Finished)),
        })
        .await
    }

    /// Gets the return value of the evaluated JavaScript.
    pub async fn join(self) -> Result<serde_json::Value, EvalError> {
        poll_fn(|cx| match self.evaluator.try_write() {
            Ok(mut evaluator) => evaluator.poll_join(cx),
            Err(_) => Poll::Ready(Err(EvalError::Finished)),
        })
        .await
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
    /// The platform does not support evaluating JavaScript.
    Unsupported,

    /// The provided JavaScript has already been ran.
    Finished,

    /// The provided JavaScript is not valid and can't be ran.
    InvalidJs(String),

    /// Represents an error communicating between JavaScript and Rust.
    Communication(String),
}

#![allow(clippy::await_holding_refcell_ref)]
#![doc = include_str!("../../docs/eval.md")]

use dioxus_core::prelude::*;
use generational_box::GenerationalBox;
use std::future::{poll_fn, Future, IntoFuture};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use super::document;

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

type EvalCreator = Rc<dyn Fn(&str) -> UseEval>;

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
    let eval_provider = document();

    Rc::new(move |script: &str| UseEval::new(eval_provider.new_evaluator(script.to_string())))
        as Rc<dyn Fn(&str) -> UseEval>
}

#[doc = include_str!("../../docs/eval.md")]
#[doc(alias = "javascript")]
pub fn eval(script: &str) -> UseEval {
    let document = use_hook(document);
    UseEval::new(document.new_evaluator(script.to_string()))
}

/// A wrapper around the target platform's evaluator that lets you send and receive data from JavaScript spawned by [`eval`].
///
#[doc = include_str!("../../docs/eval.md")]
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
        match self.evaluator.try_read() {
            Ok(evaluator) => evaluator.send(data),
            Err(_) => Err(EvalError::Finished),
        }
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
#[non_exhaustive]
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

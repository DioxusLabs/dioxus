#![doc = include_str!("../docs/eval.md")]

use crate::error::EvalError;
use generational_box::GenerationalBox;
use std::future::{poll_fn, Future, IntoFuture};
use std::pin::Pin;
use std::task::{Context, Poll};

#[doc = include_str!("../docs/eval.md")]
pub struct Eval {
    evaluator: GenerationalBox<Box<dyn Evaluator>>,
}

impl Eval {
    /// Create this eval from a dynamic evaluator
    pub fn new(evaluator: GenerationalBox<Box<dyn Evaluator + 'static>>) -> Self {
        Self { evaluator }
    }

    /// Wait until the javascript task is finished and return the result
    pub async fn join<T: serde::de::DeserializeOwned>(self) -> Result<T, EvalError> {
        let json_value = poll_fn(|cx| match self.evaluator.try_write() {
            Ok(mut evaluator) => evaluator.poll_join(cx),
            Err(_) => Poll::Ready(Err(EvalError::Finished)),
        })
        .await?;
        serde_json::from_value(json_value).map_err(EvalError::Serialization)
    }

    /// Send a message to the javascript task
    pub fn send(&self, data: impl serde::Serialize) -> Result<(), EvalError> {
        match self.evaluator.try_read() {
            Ok(evaluator) => {
                evaluator.send(serde_json::to_value(data).map_err(EvalError::Serialization)?)
            }
            Err(_) => Err(EvalError::Finished),
        }
    }

    /// Receive a message from the javascript task
    pub async fn recv<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, EvalError> {
        let json_value = poll_fn(|cx| match self.evaluator.try_write() {
            Ok(mut evaluator) => evaluator.poll_recv(cx),
            Err(_) => Poll::Ready(Err(EvalError::Finished)),
        })
        .await?;
        serde_json::from_value(json_value).map_err(EvalError::Serialization)
    }
}

impl IntoFuture for Eval {
    type Output = Result<serde_json::Value, EvalError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.join().into_future())
    }
}

impl Clone for Eval {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Eval {}

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

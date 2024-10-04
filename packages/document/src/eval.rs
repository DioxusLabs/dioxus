#![doc = include_str!("../docs/eval.md")]
use crate::error::EvalError;
use futures_util::StreamExt;
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

#[doc = include_str!("../docs/eval.md")]
pub struct Eval {
    resolve: futures_channel::oneshot::Receiver<Result<serde_json::Value, EvalError>>,
    sender: futures_channel::mpsc::UnboundedSender<Result<serde_json::Value, EvalError>>,
    receiver: futures_channel::mpsc::UnboundedReceiver<Result<serde_json::Value, EvalError>>,
}

impl Eval {
    /// Create this eval from:
    /// - A oneshot channel that, when resolved, will return the result of the eval
    /// - The sender and receiver for the eval channel
    pub fn from_parts(
        resolve: futures_channel::oneshot::Receiver<Result<serde_json::Value, EvalError>>,
        sender: futures_channel::mpsc::UnboundedSender<Result<serde_json::Value, EvalError>>,
        receiver: futures_channel::mpsc::UnboundedReceiver<Result<serde_json::Value, EvalError>>,
    ) -> Self {
        Self {
            resolve,
            sender,
            receiver,
        }
    }

    /// Wait until the javascript task is finished and return the result
    pub async fn join<T: serde::de::DeserializeOwned>(self) -> Result<T, EvalError> {
        let json_value = self
            .resolve
            .await
            .map_err(|_| EvalError::Communication("eval channel closed".to_string()))??;
        serde_json::from_value(json_value).map_err(EvalError::Serialization)
    }

    /// Send a message to the javascript task
    pub fn send(&self, data: impl serde::Serialize) -> Result<(), EvalError> {
        self.sender
            .unbounded_send(Ok(
                serde_json::to_value(data).map_err(EvalError::Serialization)?
            ))
            .map_err(|_| EvalError::Communication("eval channel closed".to_string()))
    }

    /// Receive a message from the javascript task
    pub async fn recv<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, EvalError> {
        let json_value = self
            .receiver
            .next()
            .await
            .ok_or_else(|| EvalError::Communication("eval channel closed".to_string()))??;
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

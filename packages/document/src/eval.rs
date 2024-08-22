#![doc = include_str!("../docs/eval.md")]

use crate::error::EvalError;
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

#[doc = include_str!("../docs/eval.md")]
pub struct Eval {
    rx: futures_channel::oneshot::Receiver<Result<String, EvalError>>,
}

impl Eval {
    /// Create this eval from a oneshot channel that, when resolved, will return the result of the eval
    pub fn new(rx: futures_channel::oneshot::Receiver<Result<String, EvalError>>) -> Self {
        Self { rx }
    }

    /// Create this eval and return the tx that will be used to resolve the eval
    pub fn from_parts() -> (
        futures_channel::oneshot::Sender<Result<String, EvalError>>,
        Self,
    ) {
        let (tx, rx) = futures_channel::oneshot::channel();
        (tx, Self::new(rx))
    }

    /// Poll this eval until it resolves
    pub async fn recv(self) -> Result<String, EvalError> {
        self.rx
            .await
            .map_err(|_| EvalError::Communication("eval channel closed".to_string()))?
    }

    pub async fn recv_as<T: serde::de::DeserializeOwned>(self) -> Result<T, EvalError> {
        let res = self.recv().await?;
        serde_json::from_str(&res).map_err(|e| EvalError::Deserialization(e))
    }
}

impl IntoFuture for Eval {
    type Output = Result<String, EvalError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.recv().into_future())
    }
}

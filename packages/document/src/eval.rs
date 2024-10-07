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

// #![allow(clippy::await_holding_refcell_ref)]
// #![doc = include_str!("../../docs/eval.md")]

// use dioxus_core::prelude::*;
// use generational_box::GenerationalBox;
// use std::error::Error;
// use std::fmt::Display;
// use std::future::{poll_fn, Future, IntoFuture};
// use std::pin::Pin;
// use std::rc::Rc;
// use std::task::{Context, Poll};

// use super::document;

// /// The platform's evaluator.
// pub trait Evaluator {
//     /// Sends a message to the evaluated JavaScript.
//     fn send(&self, data: serde_json::Value) -> Result<(), EvalError>;
//     /// Receive any queued messages from the evaluated JavaScript.
//     fn poll_recv(
//         &mut self,
//         context: &mut Context<'_>,
//     ) -> Poll<Result<serde_json::Value, EvalError>>;
//     /// Gets the return value of the JavaScript
//     fn poll_join(
//         &mut self,
//         context: &mut Context<'_>,
//     ) -> Poll<Result<serde_json::Value, EvalError>>;
// }

// type EvalCreator = Rc<dyn Fn(&str) -> UseEval>;

// /// Get a struct that can execute any JavaScript.
// ///
// /// # Safety
// ///
// /// Please be very careful with this function. A script with too many dynamic
// /// parts is practically asking for a hacker to find an XSS vulnerability in
// /// it. **This applies especially to web targets, where the JavaScript context
// /// has access to most, if not all of your application data.**
// #[must_use]
// pub fn eval_provider() -> EvalCreator {
//     let eval_provider = document();

//     Rc::new(move |script: &str| UseEval::new(eval_provider.new_evaluator(script.to_string())))
//         as Rc<dyn Fn(&str) -> UseEval>
// }

// #[doc = include_str!("../../docs/eval.md")]
// #[doc(alias = "javascript")]
// pub fn eval(script: &str) -> UseEval {
//     let document = use_hook(document);
//     UseEval::new(document.new_evaluator(script.to_string()))
// }

// /// A wrapper around the target platform's evaluator that lets you send and receive data from JavaScript spawned by [`eval`].
// ///
// #[doc = include_str!("../../docs/eval.md")]
// #[derive(Clone, Copy)]
// pub struct UseEval {
//     evaluator: GenerationalBox<Box<dyn Evaluator>>,
// }

// impl UseEval {
//     /// Creates a new UseEval
//     pub fn new(evaluator: GenerationalBox<Box<dyn Evaluator + 'static>>) -> Self {
//         Self { evaluator }
//     }

//     /// Sends a [`serde_json::Value`] to the evaluated JavaScript.
//     pub fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
//         match self.evaluator.try_read() {
//             Ok(evaluator) => evaluator.send(data),
//             Err(_) => Err(EvalError::Finished),
//         }
//     }

//     /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
//     pub async fn recv(&mut self) -> Result<serde_json::Value, EvalError> {
//         poll_fn(|cx| match self.evaluator.try_write() {
//             Ok(mut evaluator) => evaluator.poll_recv(cx),
//             Err(_) => Poll::Ready(Err(EvalError::Finished)),
//         })
//         .await
//     }

//     /// Gets the return value of the evaluated JavaScript.
//     pub async fn join(self) -> Result<serde_json::Value, EvalError> {
//         poll_fn(|cx| match self.evaluator.try_write() {
//             Ok(mut evaluator) => evaluator.poll_join(cx),
//             Err(_) => Poll::Ready(Err(EvalError::Finished)),
//         })
//         .await
//     }
// }

// impl IntoFuture for UseEval {
//     type Output = Result<serde_json::Value, EvalError>;
//     type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

//     fn into_future(self) -> Self::IntoFuture {
//         Box::pin(self.join())
//     }
// }

// /// Represents an error when evaluating JavaScript
// #[derive(Debug)]
// #[non_exhaustive]
// pub enum EvalError {
//     /// The platform does not support evaluating JavaScript.
//     Unsupported,

//     /// The provided JavaScript has already been ran.
//     Finished,

//     /// The provided JavaScript is not valid and can't be ran.
//     InvalidJs(String),

//     /// Represents an error communicating between JavaScript and Rust.
//     Communication(String),
// }

// impl Display for EvalError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             EvalError::Unsupported => write!(f, "EvalError::Unsupported - eval is not supported on the current platform"),
//             EvalError::Finished => write!(f, "EvalError::Finished - eval has already ran"),
//             EvalError::InvalidJs(_) => write!(f, "EvalError::InvalidJs - the provided javascript is invalid"),
//             EvalError::Communication(_) => write!(f, "EvalError::Communication - there was an error trying to communicate with between javascript and rust"),
//         }
//     }
// }

// impl Error for EvalError {}

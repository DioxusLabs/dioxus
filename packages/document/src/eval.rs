#![allow(clippy::await_holding_refcell_ref)]
#![doc = include_str!("../docs/eval.md")]

use dioxus_core::prelude::*;
// use generational_box::GenerationalBox;
use std::error::Error;
use std::fmt::Display;
use std::future::{poll_fn, Future, IntoFuture};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use super::{document, Document};

#[doc = include_str!("../docs/eval.md")]
#[doc(alias = "javascript")]
pub fn eval(script: &str) -> Eval {
    todo!()
    //     let document = use_hook(document);
    //     UseEval::new(document.new_evaluator(script.to_string()))
}

#[derive(Clone, Copy)]
pub struct Eval {}

/// A wrapper around the target platform's evaluator that lets you send and receive data from JavaScript spawned by [`eval`].
///
#[doc = include_str!("../docs/eval.md")]
#[derive(Clone, Copy)]
pub struct UseEval {
    // evaluator: Rc<dyn Document>,
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

impl Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::Unsupported => write!(f, "EvalError::Unsupported - eval is not supported on the current platform"),
            EvalError::Finished => write!(f, "EvalError::Finished - eval has already ran"),
            EvalError::InvalidJs(_) => write!(f, "EvalError::InvalidJs - the provided javascript is invalid"),
            EvalError::Communication(_) => write!(f, "EvalError::Communication - there was an error trying to communicate with between javascript and rust"),
        }
    }
}

impl Error for EvalError {}

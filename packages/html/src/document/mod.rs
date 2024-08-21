// API inspired by Reacts implementation of head only elements. We use components here instead of elements to simplify internals.

use std::{
    rc::Rc,
    task::{Context, Poll},
};

// use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};

// mod bindings;
// #[allow(unused)]
// pub use bindings::*;

// pub mod head;
// pub use head::{
//     Meta, MetaProps, Script, ScriptProps, Style, StyleProps, Stylesheet, Title, TitleProps,
// };

// /// The default No-Op document
// pub struct NoOpDocument;

// impl Document for NoOpDocument {
//     fn new_evaluator(&self, _js: String) -> GenerationalBox<Box<dyn Evaluator>> {
//         tracing::error!("Eval is not supported on this platform. If you are using dioxus fullstack, you can wrap your code with `client! {{}}` to only include the code that runs eval in the client bundle.");
//         UnsyncStorage::owner().insert(Box::new(NoOpEvaluator))
//     }

//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }
// }

// struct NoOpEvaluator;
// impl Evaluator for NoOpEvaluator {
//     fn send(&self, _data: serde_json::Value) -> Result<(), EvalError> {
//         Err(EvalError::Unsupported)
//     }
//     fn poll_recv(
//         &mut self,
//         _context: &mut Context<'_>,
//     ) -> Poll<Result<serde_json::Value, EvalError>> {
//         Poll::Ready(Err(EvalError::Unsupported))
//     }
//     fn poll_join(
//         &mut self,
//         _context: &mut Context<'_>,
//     ) -> Poll<Result<serde_json::Value, EvalError>> {
//         Poll::Ready(Err(EvalError::Unsupported))
//     }
// }

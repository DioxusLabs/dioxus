// API inspired by Reacts implementation of head only elements. We use components here instead of elements to simplify internals.

use std::{
    rc::Rc,
    task::{Context, Poll},
};

use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};

#[allow(unused)]
mod eval;
pub use eval::*;

pub mod head;
pub use head::{Meta, MetaProps, Script, ScriptProps, Style, StyleProps, Title, TitleProps};

fn format_attributes(attributes: &[(&str, String)]) -> String {
    let mut formatted = String::from("[");
    for (key, value) in attributes {
        formatted.push_str(&format!("[{key:?}, {value:?}],"));
    }
    if formatted.ends_with(',') {
        formatted.pop();
    }
    formatted.push(']');
    formatted
}

fn create_element_in_head(
    tag: &str,
    attributes: &[(&str, String)],
    children: Option<String>,
) -> String {
    let helpers = include_str!("../js/head.js");
    let attributes = format_attributes(attributes);
    let children = children
        .map(|c| format!("\"{c}\""))
        .unwrap_or("null".to_string());
    format!(r#"{helpers};window.createElementInHead("{tag}", {attributes}, {children});"#)
}

/// A provider for document-related functionality. By default most methods are driven through [`eval`].
pub trait Document {
    /// Create a new evaluator for the document that evaluates JavaScript and facilitates communication between JavaScript and Rust.
    fn new_evaluator(&self, js: String) -> GenerationalBox<Box<dyn Evaluator>>;

    /// Get a reference to the document as `dyn Any`
    fn as_any(&self) -> &dyn std::any::Any;
}

/// The default No-Op document
pub struct NoOpDocument;

impl Document for NoOpDocument {
    fn new_evaluator(&self, _js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        tracing::error!("Eval is not supported on this platform. If you are using dioxus fullstack, you can wrap your code with `client! {{}}` to only include the code that runs eval in the client bundle.");
        UnsyncStorage::owner().insert(Box::new(NoOpEvaluator))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct NoOpEvaluator;
impl Evaluator for NoOpEvaluator {
    fn send(&self, _data: serde_json::Value) -> Result<(), EvalError> {
        Err(EvalError::Unsupported)
    }
    fn poll_recv(
        &mut self,
        _context: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, EvalError>> {
        Poll::Ready(Err(EvalError::Unsupported))
    }
    fn poll_join(
        &mut self,
        _context: &mut Context<'_>,
    ) -> Poll<Result<serde_json::Value, EvalError>> {
        Poll::Ready(Err(EvalError::Unsupported))
    }
}

/// Get the document provider for the current platform or a no-op provider if the platform doesn't document functionality.
pub fn document() -> Rc<dyn Document> {
    dioxus_core::prelude::try_consume_context::<Rc<dyn Document>>()
        // Create a NoOp provider that always logs an error when trying to evaluate
        // That way, we can still compile and run the code without a real provider
        .unwrap_or_else(|| Rc::new(NoOpDocument) as Rc<dyn Document>)
}

use async_channel::Receiver;
use async_trait::async_trait;
use dioxus_core::ScopeState;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::rc::Rc;

/// A struct that implements EvalProvider is sent through [`ScopeState`]'s provide_context function
/// so that [`use_eval`] can provide a platform agnostic interface for evaluating JavaScript code.
pub trait EvalProvider {
    fn new_evaluator(&self, js: String) -> Box<dyn Evaluator>;
}

/// The platform's evaluator.
#[async_trait(?Send)]
pub trait Evaluator {
    /// Runs the evaluated JavaScript.
    fn run(&mut self) -> Result<(), EvalError>;
    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError>;
    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    fn receiver(&mut self) -> Receiver<serde_json::Value>;
    /// Cleans up any evaluation artifacts.
    fn done(&mut self);
}

/// Get a struct that can execute any JavaScript.
///
/// # Safety
///
/// Please be very careful with this function. A script with too many dynamic
/// parts is practically asking for a hacker to find an XSS vulnerability in
/// it. **This applies especially to web targets, where the JavaScript context
/// has access to most, if not all of your application data.**
pub fn use_eval(cx: &ScopeState) -> &Rc<dyn Fn(&str) -> UseEval> {
    cx.use_hook(|| {
        let eval_provider = cx
            .consume_context::<Rc<dyn EvalProvider>>()
            .expect("evaluator not provided");

        Rc::new(move |script: &str| {
            let evaluator = eval_provider.new_evaluator(script.to_string());
            UseEval::new(evaluator)
        }) as Rc<dyn Fn(&str) -> UseEval>
    })
}

/// A wrapper around the target platform's evaluator.
pub struct UseEval {
    evaluator: Box<dyn Evaluator + 'static>,
}

impl UseEval {
    /// Creates a new UseEval
    pub fn new(evaluator: Box<dyn Evaluator + 'static>) -> Self {
        Self { evaluator }
    }

    /// Runs the evaluated JavaScript.
    pub fn run(&mut self) -> Result<(), EvalError> {
        self.evaluator.run()
    }

    /// Sends a [`serde_json::Value`] to the evaluated JavaScript.
    pub fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        self.evaluator.send(data)
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    pub fn receiver(&mut self) -> Receiver<serde_json::Value> {
        self.evaluator.receiver()
    }

    /// Cleans up any evaluation artifacts.
    pub fn done(&mut self) {
        self.evaluator.done();
    }
}

impl IntoFuture for UseEval {
    type Output = Result<serde_json::Value, EvalError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(mut self) -> Self::IntoFuture {
        Box::pin(async move {
            self.run()?;
            let data = self.receiver().recv().await.map_err(|_| {
                EvalError::Communication("failed to receive value from js".to_string())
            })?;

            Ok(data)
        })
    }
}

impl Drop for UseEval {
    fn drop(&mut self) {
        self.done();
    }
}

/// Represents an error when evaluating JavaScript
#[derive(Debug)]
pub enum EvalError {
    /// The evaluator's ``run`` method hasn't been called.
    /// Messages cannot be received at this time.
    NotRan,
    /// The provides JavaScript is not valid and can't be ran.
    InvalidJs(String),
    /// Represents an error communicating between JavaScript and Rust.
    Communication(String),
}

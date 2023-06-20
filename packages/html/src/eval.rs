use async_trait::async_trait;
use dioxus_core::ScopeState;
use futures_util::Stream;
use std::{
    collections::VecDeque,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

/// A struct that implements EvalProvider is sent through [`ScopeState`]'s provide_context function
/// so that [`use_eval`] can provide a platform agnostic interface for evaluating JavaScript code.
pub trait EvalProvider {
    fn new_evaluator(&self, cx: &ScopeState, js: String) -> Box<dyn Evaluator>;
}

/// The platform's evaluator.
#[async_trait(?Send)]
pub trait Evaluator {
    /// Runs the evaluated JavaScript.
    fn run(&mut self) -> Result<(), EvalError>;
    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError>;
    /// Receives a message from the evaluated JavaScript.
    async fn recv(&mut self) -> Result<serde_json::Value, EvalError>;
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
pub fn use_eval<S: ToString>(cx: &ScopeState, js: S) -> &mut UseEval {
    cx.use_hook(|| {
        let eval_provider = cx
            .consume_context::<Rc<dyn EvalProvider>>()
            .expect("evaluator not provided");

        let evaluator = eval_provider.new_evaluator(cx, js.to_string());

        UseEval::new(evaluator)
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

    /// Receives a [`serde_json::Value`] from the evaluated JavaScript.
    pub async fn recv(&mut self) -> Result<serde_json::Value, EvalError> {
        self.evaluator.recv().await
    }

    /// Cleans up any evaluation artifacts.
    pub fn done(&mut self) {
        self.evaluator.done();
    }
}

/// MessageQueue is a wrapper around a [`VecDeque`] that implements future-util's [`Stream`] trait.
#[derive(Debug)]
pub struct MessageQueue {
    queue: VecDeque<serde_json::Value>,
}

impl MessageQueue {
    /// Creates a new MessageQueue.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Pops an item off the front.
    pub fn pop(&mut self) -> Option<serde_json::Value> {
        self.queue.pop_front()
    }

    /// Pushes an item onto the back.
    pub fn push(&mut self, value: serde_json::Value) {
        self.queue.push_back(value);
    }
}

impl Stream for MessageQueue {
    type Item = serde_json::Value;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(value) = self.pop() {
            Poll::Ready(Some(value))
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
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

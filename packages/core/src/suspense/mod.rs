//! Suspense allows you to render a placeholder while nodes are waiting for data in the background
//!
//! During suspense on the server:
//! - Rebuild once
//! - Send page with loading placeholders down to the client
//! - loop
//!   - Poll (only) suspended futures
//!   - If a scope is marked as dirty and that scope is a suspense boundary, under a suspended boundary, or the suspense placeholder, rerun the scope
//!     - If it is a different scope, ignore it and warn the user
//!   - Rerender the scope on the server and send down the nodes under a hidden div with serialized data
//!
//! During suspense on the web:
//! - Rebuild once without running server futures
//! - Rehydrate the placeholders that were initially sent down. At this point, no suspense nodes are resolved so the client and server pages should be the same
//! - loop
//!   - Wait for work or suspense data
//!   - If suspense data comes in
//!     - replace the suspense placeholder
//!     - get any data associated with the suspense placeholder and rebuild nodes under the suspense that was resolved
//!     - rehydrate the suspense placeholders that were at that node
//!   - If work comes in
//!     - Just do the work; this may remove suspense placeholders that the server hasn't yet resolved. If we see new data come in from the server about that node, ignore it
//!
//! Generally suspense placeholders should not be stateful because they are driven from the server. If they are stateful and the client renders something different, hydration will fail.

mod component;
pub use component::*;

use crate::innerlude::*;
use std::{
    cell::{Cell, Ref, RefCell},
    fmt::Debug,
    rc::Rc,
};

/// A context with information about suspended components
#[derive(Debug, Clone)]
pub struct SuspenseContext {
    pub(crate) inner: Rc<SuspenseBoundaryInner>,
}

/// A boundary that will capture any errors from child components
pub(crate) struct SuspenseBoundaryInner {
    pub(crate) suspended_tasks: RefCell<Vec<SuspendedFuture>>,

    pub(crate) id: Cell<ScopeId>,

    /// The nodes that are suspended under this boundary
    pub(crate) suspended_nodes: RefCell<Option<VNode>>,

    /// On the server, you can only resolve a suspense boundary once. This is used to track if the suspense boundary has been resolved and if it should be frozen
    pub(crate) frozen: Cell<bool>,

    /// Closures queued to run after the suspense boundary is resolved
    pub(crate) after_suspense_resolved: RefCell<Vec<Box<dyn FnOnce()>>>,
}

impl SuspenseContext {
    /// Create a new suspense boundary in a specific scope
    pub(crate) fn new() -> Self {
        Self {
            inner: Rc::new(SuspenseBoundaryInner {
                suspended_tasks: RefCell::new(vec![]),
                id: Cell::new(ScopeId::ROOT),
                suspended_nodes: Default::default(),
                frozen: Default::default(),
                after_suspense_resolved: Default::default(),
            }),
        }
    }

    /// Mount the context in a specific scope
    pub(crate) fn mount(&self, scope: ScopeId) {
        self.inner.id.set(scope);
    }

    /// Get the suspense boundary's suspended nodes
    pub fn suspended_nodes(&self) -> Option<VNode> {
        self.inner
            .suspended_nodes
            .borrow()
            .as_ref()
            .map(|node| node.clone())
    }

    /// Set the suspense boundary's suspended nodes
    pub(crate) fn set_suspended_nodes(&self, suspended_nodes: VNode) {
        self.inner
            .suspended_nodes
            .borrow_mut()
            .replace(suspended_nodes);
    }

    /// Take the suspense boundary's suspended nodes
    pub(crate) fn take_suspended_nodes(&self) -> Option<VNode> {
        self.inner.suspended_nodes.borrow_mut().take()
    }

    /// Check if the suspense boundary is resolved and frozen
    pub fn frozen(&self) -> bool {
        self.inner.frozen.get()
    }

    /// Resolve the suspense boundary on the server and freeze it to prevent future reruns of any child nodes of the suspense boundary
    pub fn freeze(&self) {
        self.inner.frozen.set(true);
    }

    /// Check if there are any suspended tasks
    pub fn has_suspended_tasks(&self) -> bool {
        !self.inner.suspended_tasks.borrow().is_empty()
    }

    /// Check if the suspense boundary is currently rendered as suspended
    pub fn is_suspended(&self) -> bool {
        self.inner.suspended_nodes.borrow().is_some()
    }

    /// Add a suspended task, returning true if it was added and false if it was already present
    pub(crate) fn add_suspended_task(&self, task: SuspendedFuture) -> bool {
        let mut tasks = self.inner.suspended_tasks.borrow_mut();

        if tasks.iter().any(|t| t == &task) {
            return false;
        }

        tasks.push(task);

        true
    }

    /// Remove a suspended task
    pub(crate) fn remove_suspended_task(&self, task: Task) {
        self.inner
            .suspended_tasks
            .borrow_mut()
            .retain(|t| t.task() != task);
    }

    /// Get all suspended tasks
    pub fn suspended_futures(&self) -> Ref<'_, [SuspendedFuture]> {
        Ref::map(self.inner.suspended_tasks.borrow(), |tasks| {
            tasks.as_slice()
        })
    }

    /// Run a closure after suspense is resolved
    pub fn after_suspense_resolved(&self, callback: impl FnOnce() + 'static) {
        let mut closures = self.inner.after_suspense_resolved.borrow_mut();
        closures.push(Box::new(callback));
    }

    /// Run all closures that were queued to run after suspense is resolved
    pub(crate) fn run_resolved_closures(&self, runtime: &Runtime) {
        runtime.while_not_rendering(|| {
            self.inner
                .after_suspense_resolved
                .borrow_mut()
                .drain(..)
                .for_each(|f| f());
        })
    }
}

impl PartialEq for SuspenseContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Debug for SuspenseBoundaryInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuspenseBoundaryInner")
            .field("suspended_tasks", &self.suspended_tasks)
            .field("id", &self.id)
            .field("suspended_nodes", &self.suspended_nodes)
            .field("frozen", &self.frozen)
            .finish()
    }
}

/// A task spawned with `spawn` that has suspended its tree.
#[derive(Clone, PartialEq, Debug, Hash, Eq)]
pub struct SuspendedFuture(TaskId);

impl SuspendedFuture {
    /// Create a new suspended future
    pub fn new(task: Task) -> Self {
        Self(task.id)
    }

    /// Get the task that was suspended
    pub fn task(&self) -> Task {
        Task::from_id(self.0)
    }
}

impl std::fmt::Display for SuspendedFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SuspendedFuture {{ task: {:?} }}", self.task())
    }
}

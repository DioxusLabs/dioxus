use crate::innerlude::*;
use std::{cell::RefCell, fmt::Debug, rc::Rc};

#[derive(Clone, PartialEq, Debug)]
pub struct SuspendedFuture {
    task: Task,
    pub(crate) placeholder: VNode,
}

impl SuspendedFuture {
    pub fn new(task: Task) -> Self {
        Self {
            task,
            placeholder: VNode::placeholder(),
        }
    }

    pub fn suspense_placeholder(self) -> VNode {
        self.placeholder.clone()
    }

    pub fn with_placeholder(mut self, placeholder: VNode) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn task(&self) -> Task {
        self.task
    }

    /// Clone the future while retaining the mounted information of the future
    pub(crate) fn clone_mounted(&self) -> Self {
        Self {
            task: self.task,
            placeholder: self.placeholder.clone_mounted(),
        }
    }
}

/// Provide an error boundary to catch errors from child components
pub fn use_suspense_boundary() -> SuspenseBoundary {
    use_hook(|| provide_context(SuspenseBoundary::new()))
}

/// A boundary that will capture any errors from child components
#[derive(Debug, Clone, Default)]
pub struct SuspenseBoundary {
    inner: Rc<SuspenseBoundaryInner>,
}

impl SuspenseBoundary {
    /// Create a new suspense boundary
    pub fn new() -> Self {
        Self::default()
    }
}

/// A boundary that will capture any errors from child components
#[derive(Debug, Default)]
pub struct SuspenseBoundaryInner {
    suspended_tasks: RefCell<Vec<Task>>,
}

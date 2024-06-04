mod component;
pub use component::*;

use crate::innerlude::*;
use std::{
    cell::{Ref, RefCell},
    fmt::Debug,
    rc::Rc,
};

/// A task that has been suspended which may have an optional loading placeholder
#[derive(Clone, PartialEq, Debug)]
pub struct SuspendedFuture {
    origin: ScopeId,
    task: Task,
    pub(crate) placeholder: VNode,
}

impl SuspendedFuture {
    /// Create a new suspended future
    pub fn new(task: Task) -> Self {
        Self {
            task,
            origin: current_scope_id().expect("to be in a dioxus runtime"),
            placeholder: VNode::placeholder(),
        }
    }

    /// Get a placeholder to display while the future is suspended
    pub fn suspense_placeholder(&self) -> Option<VNode> {
        if self.placeholder == VNode::placeholder() {
            None
        } else {
            Some(self.placeholder.clone())
        }
    }

    /// Set a new placeholder the SuspenseBoundary may use to display while the future is suspended
    pub fn with_placeholder(mut self, placeholder: VNode) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// Get the task that was suspended
    pub fn task(&self) -> Task {
        self.task
    }

    /// Clone the future while retaining the mounted information of the future
    pub(crate) fn clone_mounted(&self) -> Self {
        Self {
            task: self.task,
            origin: self.origin,
            placeholder: self.placeholder.clone_mounted(),
        }
    }
}

/// A boundary that freezes rendering for all child nodes.
/// This should be created as a child of [`SuspenseContext`] and used to wrap all child nodes.
#[derive(Debug, Clone, Default)]
pub struct FrozenContext {
    inner: Rc<SuspenseBoundaryInner>,
}

impl FrozenContext {
    pub(crate) fn frozen(&self) -> bool {
        !self.inner.suspended_tasks.borrow().is_empty()
    }
}

/// A boundary that will capture any errors from child components
/// NOTE: this will not prevent rendering in child components. [`FrozenContext`] should be used instead.
#[derive(Debug, Clone, Default)]
pub struct SuspenseContext {
    inner: Rc<SuspenseBoundaryInner>,
}

impl PartialEq for SuspenseContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl SuspenseContext {
    /// Create a new suspense boundary
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new suspense boundary in a specific scope
    pub(crate) fn new_in_scope(scope: ScopeId) -> Self {
        Self {
            inner: Rc::new(SuspenseBoundaryInner {
                suspended_tasks: RefCell::new(vec![]),
                id: scope,
            }),
        }
    }

    /// Get a frozen context that will freeze rendering for all child nodes
    pub fn freeze(self) -> FrozenContext {
        FrozenContext { inner: self.inner }
    }

    /// Add a suspended task
    pub(crate) fn add_suspended_task(&self, task: SuspendedFuture) {
        self.inner.suspended_tasks.borrow_mut().push(task);
        self.inner.id.needs_update();
    }

    /// Remove a suspended task
    pub(crate) fn remove_suspended_task(&self, task: Task) {
        self.inner
            .suspended_tasks
            .borrow_mut()
            .retain(|t| t.task != task);
        self.inner.id.needs_update();
    }

    /// Get all suspended tasks
    pub fn suspended_futures(&self) -> Ref<[SuspendedFuture]> {
        Ref::map(self.inner.suspended_tasks.borrow(), |tasks| {
            tasks.as_slice()
        })
    }

    /// Get the first suspended task with a loading placeholder
    pub fn suspense_placeholder(&self) -> Option<Element> {
        self.inner
            .suspended_tasks
            .borrow()
            .iter()
            .find_map(|task| task.suspense_placeholder())
            .map(std::result::Result::Ok)
    }
}

/// A boundary that will capture any errors from child components
#[derive(Debug)]
pub struct SuspenseBoundaryInner {
    suspended_tasks: RefCell<Vec<SuspendedFuture>>,
    id: ScopeId,
}

impl Default for SuspenseBoundaryInner {
    fn default() -> Self {
        Self {
            suspended_tasks: RefCell::new(Vec::new()),
            id: current_scope_id().expect("to be in a dioxus runtime"),
        }
    }
}

/// Provides context methods to [`Result<T, RenderError>`] to show loading indicators
///
/// This trait is sealed and cannot be implemented outside of dioxus-core
pub trait SuspenseExtension<T>: private::Sealed {
    /// Add a loading indicator if the result is suspended
    fn with_loading_placeholder(
        self,
        display_placeholder: impl FnOnce() -> Element,
    ) -> std::result::Result<T, RenderError>;
}

impl<T> SuspenseExtension<T> for std::result::Result<T, RenderError> {
    fn with_loading_placeholder(
        self,
        display_placeholder: impl FnOnce() -> Element,
    ) -> std::result::Result<T, RenderError> {
        if let Err(RenderError::Suspended(suspense)) = self {
            Err(RenderError::Suspended(suspense.with_placeholder(
                display_placeholder().unwrap_or_default(),
            )))
        } else {
            self
        }
    }
}

pub(crate) mod private {
    use super::*;

    pub trait Sealed {}

    impl<T> Sealed for std::result::Result<T, RenderError> {}
}

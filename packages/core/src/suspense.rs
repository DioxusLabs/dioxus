use crate::innerlude::*;
use std::{
    cell::{Ref, RefCell},
    fmt::Debug,
    rc::Rc,
};

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

    pub fn suspense_placeholder(&self) -> Option<VNode> {
        if self.placeholder == VNode::placeholder() {
            None
        } else {
            Some(self.placeholder.clone())
        }
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
    pub fn suspense_placeholder(&self) -> Option<VNode> {
        self.inner
            .suspended_tasks
            .borrow()
            .iter()
            .find_map(|task| task.suspense_placeholder())
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
pub trait SuspenseContext<T>: private::Sealed {
    /// Add a loading indicator if the result is suspended
    fn with_loading_placeholder(
        self,
        display_placeholder: impl FnOnce() -> Element,
    ) -> std::result::Result<T, RenderError>;
}

impl<T> SuspenseContext<T> for std::result::Result<T, RenderError> {
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

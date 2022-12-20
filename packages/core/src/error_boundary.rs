use std::cell::RefCell;

use crate::ScopeId;

/// A boundary that will capture any errors from child components
#[allow(dead_code)]
pub struct ErrorBoundary {
    error: RefCell<Option<ScopeId>>,
    id: ScopeId,
}

impl ErrorBoundary {
    pub fn new(id: ScopeId) -> Self {
        Self {
            error: RefCell::new(None),
            id,
        }
    }
}

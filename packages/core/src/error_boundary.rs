use std::{cell::RefCell, rc::Rc};

use crate::{ScopeId, ScopeState};

pub struct ErrorContext {
    error: RefCell<Option<(anyhow::Error, ScopeId)>>,
}

/// Catch all errors from the children and bubble them up to this component
///
/// Returns the error and scope that caused the error
pub fn use_catch_error(cx: &ScopeState) -> Option<&(anyhow::Error, ScopeId)> {
    let err_ctx = use_error_context(cx);

    let out = cx.use_hook(|| None);

    if let Some(error) = err_ctx.error.take() {
        *out = Some(error);
    }

    out.as_ref()
}

/// Create a new error context at this component.
///
/// This component will start to catch any errors that occur in its children.
pub fn use_error_context(cx: &ScopeState) -> &ErrorContext {
    cx.use_hook(|| cx.provide_context(Rc::new(ErrorContext { error: None.into() })))
}

use std::{
    backtrace::Backtrace,
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::innerlude::*;

#[derive(Clone, PartialEq, Debug)]
pub enum RenderError {
    /// The render function returned early
    Aborted(CapturedError),

    /// The component was suspended
    Suspended(SuspendedFuture),
}

impl Default for RenderError {
    fn default() -> Self {
        struct RenderAbortedEarly;
        impl Debug for RenderAbortedEarly {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Render aborted early")
            }
        }
        impl Display for RenderAbortedEarly {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Render aborted early")
            }
        }
        impl std::error::Error for RenderAbortedEarly {}
        Self::Aborted(RenderAbortedEarly.into())
    }
}

impl RenderError {}

impl Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aborted(e) => write!(f, "Render aborted: {e}"),
            Self::Suspended(e) => write!(f, "Component suspended: {e:?}"),
        }
    }
}

impl<E: std::error::Error + 'static> From<E> for RenderError {
    fn from(e: E) -> Self {
        Self::Aborted(CapturedError::from(e))
    }
}

/// An extension for Result<T, RenderError> types with helpful methods for rendering a placeholder or adding context to errors
trait RenderErrorExt {
    fn render_with_placeholder<T>(self, placeholder: T) -> Result<T, RenderError>
    where
        T: Into<VNode>;

    fn context<T>(self, context: T) -> Result<T, RenderError>
    where
        T: Display;
}

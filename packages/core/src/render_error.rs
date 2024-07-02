use std::fmt::{Debug, Display};

use crate::innerlude::*;

/// An error that can occur while rendering a component
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

impl From<CapturedError> for RenderError {
    fn from(e: CapturedError) -> Self {
        RenderError::Aborted(e)
    }
}

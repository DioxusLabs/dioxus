use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::innerlude::*;

/// An error that can occur while rendering a component
#[derive(Debug)]
pub enum RenderError {
    /// The render function returned early due to an error
    Error(Arc<Error>),

    /// The component was suspended
    Suspended(SuspendedFuture),
}

impl Clone for RenderError {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl PartialEq for RenderError {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
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
        Self::Error(Arc::new(RenderAbortedEarly.into()))
    }
}

impl Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error(e) => write!(f, "Render aborted: {e}"),
            Self::Suspended(e) => write!(f, "Component suspended: {e:?}"),
        }
    }
}

impl<E: Into<Error>> From<E> for RenderError {
    fn from(e: E) -> Self {
        todo!()
        // Self::Aborted(CapturedError::from(e))
    }
}

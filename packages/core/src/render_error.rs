use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::innerlude::*;

/// An error that can occur while rendering a component
#[derive(Debug, Clone, PartialEq)]
pub enum RenderError {
    /// The render function returned early due to an error.
    ///
    /// We captured the error, wrapped it in an Arc, and stored it here. You can no longer modify the error,
    /// but you can cheaply pass it around.
    Error(CapturedError),

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
        Self::Error(CapturedError(Arc::new(RenderAbortedEarly.into())))
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
        Self::Error(CapturedError(Arc::new(e.into())))
    }
}

/// An `anyhow::Error` wrapped in an `Arc` so it can be cheaply cloned and passed around.
#[derive(Debug, Clone)]
pub struct CapturedError(Arc<Error>);
impl std::ops::Deref for CapturedError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E: Into<Error>> From<E> for CapturedError {
    fn from(e: E) -> Self {
        Self(Arc::new(e.into()))
    }
}

impl std::fmt::Display for CapturedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq for CapturedError {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

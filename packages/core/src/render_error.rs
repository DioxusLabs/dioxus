use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

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
pub struct CapturedError(pub Arc<Error>);
impl CapturedError {
    /// Create a `CapturedError` from anything that implements `Display`.
    pub fn from_display(t: impl Display) -> Self {
        Self(Arc::new(anyhow::anyhow!(t.to_string())))
    }
}
impl Serialize for CapturedError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for CapturedError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_display(s))
    }
}

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

// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // /// Android-specific error
    // #[cfg(target_os = "android")]
    // #[error("Android error: {0}")]
    // Android(#[from] jni::errors::Error),
    /// iOS-specific error
    #[cfg(target_os = "ios")]
    #[error("iOS error: {0}")]
    Ios(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Platform bridge error
    #[error("Platform bridge error: {0}")]
    PlatformBridge(String),

    /// Location services are disabled
    #[error("Location services are disabled")]
    LocationServicesDisabled,

    /// Permission denied
    #[error("Permission denied")]
    PermissionDenied,

    /// Location unavailable
    #[error("Location unavailable: {0}")]
    LocationUnavailable(String),

    /// Timeout waiting for location
    #[error("Timeout waiting for location")]
    Timeout,

    /// Live Activity error (iOS 16.1+)
    #[error("Live Activity error: {0}")]
    LiveActivity(String),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::PlatformBridge(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::PlatformBridge(s)
    }
}

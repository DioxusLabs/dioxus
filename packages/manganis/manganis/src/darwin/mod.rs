//! Darwin (iOS/macOS) shared utilities for objc2-based APIs
//!
//! This module provides shared utilities for both iOS and macOS platforms
//! since they share the same Objective-C runtime and threading requirements
//! through objc2.

/// manager
pub mod manager;

pub use manager::*;

/// Re-export MainThreadMarker for convenience
pub use objc2::MainThreadMarker;

/// Macro helpers for FFI code generation
pub mod macro_helpers {
    pub use crate::macro_helpers::copy_bytes;
}

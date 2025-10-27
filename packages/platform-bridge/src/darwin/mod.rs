//! Darwin (iOS/macOS) shared utilities for objc2-based APIs
//!
//! This module provides shared utilities for both iOS and macOS platforms
//! since they share the same Objective-C runtime and threading requirements
//! through objc2.

pub mod manager;

pub use manager::*;

/// Re-export MainThreadMarker for convenience
pub use objc2::MainThreadMarker;

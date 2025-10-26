//! Core utilities and abstractions for Dioxus mobile platform APIs
//!
//! This crate provides common patterns and utilities for implementing
//! cross-platform mobile APIs in Dioxus applications. It handles the
//! boilerplate for JNI (Android) and objc2 (iOS) bindings, build scripts,
//! and platform-specific resource management.

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_os = "android")]
pub use android::*;

#[cfg(target_os = "ios")]
pub use ios::*;

/// Re-export commonly used types for convenience
#[cfg(target_os = "android")]
pub use jni;

#[cfg(target_os = "ios")]
pub use objc2;

/// Re-export the java_plugin! macro when metadata feature is enabled
#[cfg(all(feature = "metadata", target_os = "android"))]
pub use mobile_core_macro::java_plugin;

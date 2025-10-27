//! FFI utilities and plugin metadata for Dioxus mobile platform APIs
//!
//! This crate provides common patterns and utilities for implementing
//! mobile platform APIs in Dioxus applications. It handles the
//! boilerplate for JNI (Android) and objc2 (iOS/macOS) bindings, build scripts,
//! and platform-specific resource management.

#[cfg(target_os = "android")]
pub mod android;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod darwin;

#[cfg(target_os = "android")]
pub use android::*;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub use darwin::*;

/// Re-export commonly used types for convenience
#[cfg(target_os = "android")]
pub use jni;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub use objc2;

/// Re-export the android_plugin! macro when metadata feature is enabled
#[cfg(all(feature = "metadata", target_os = "android"))]
pub use platform_bridge_macro::android_plugin;

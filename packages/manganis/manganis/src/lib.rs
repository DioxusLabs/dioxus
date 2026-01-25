#![doc = include_str!("../README.md")]
// #![deny(missing_docs)]

#[doc(hidden)]
pub mod macro_helpers;
pub use manganis_macro::asset;
pub use manganis_macro::css_module;
pub use manganis_macro::option_asset;
pub use manganis_macro::widget;

pub use manganis_core::{
    // Core asset types
    Asset,
    AssetOptions,
    AssetVariant,
    BundledAsset,
    // Standard asset options
    CssAssetOptions,
    CssModuleAssetOptions,
    FolderAssetOptions,
    ImageAssetOptions,
    ImageFormat,
    ImageSize,
    JsAssetOptions,
};

// Re-export metadata types for FFI and sidecar macros
pub use manganis_core::SwiftPackageMetadata;
pub use manganis_core::{AppleWidgetExtensionMetadata, SymbolData};

// FFI utilities and plugin metadata for Dioxus mobile platform APIs
//
// This crate provides common patterns and utilities for implementing
// mobile platform APIs in Dioxus applications. It handles the
// boilerplate for JNI (Android) and objc2 (iOS/macOS) bindings, build scripts,
// and platform-specific resource management.

/// Android platform utilities
#[doc(hidden)]
#[cfg(any(target_os = "android"))]
pub mod android;

/// Darwin (iOS/macOS) platform utilities
#[doc(hidden)]
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod darwin;

#[cfg(target_os = "android")]
pub use android::*;

// Export darwin module for iOS, macOS, and when metadata feature is enabled (for FFI macro)
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub use darwin::*;

/// Re-export commonly used types for convenience
#[cfg(target_os = "android")]
pub use jni;

// Re-export objc2 for FFI macro generated code
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub use objc2;

/// Re-export the ffi attribute macro for native FFI bindings
/// This macro generates direct FFI bindings between Rust and native platforms (Swift/Kotlin)
pub use manganis_macro::ffi;

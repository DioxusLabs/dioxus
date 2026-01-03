#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

#[doc(hidden)]
pub mod macro_helpers;
pub use manganis_macro::asset;
pub use manganis_macro::css_module;
pub use manganis_macro::option_asset;

pub use manganis_core::{
    Asset, AssetOptions, AssetVariant, BundledAsset, CssAssetOptions, CssModuleAssetOptions,
    FolderAssetOptions, ImageAssetOptions, ImageFormat, ImageSize, JsAssetOptions,
};

pub mod permissions {

    //! # Permissions
    //!
    //! A cross-platform permission management system with linker-based collection.
    //!
    //! This crate provides a unified API for declaring permissions across supported platforms
    //! (Android, iOS, macOS) and embeds them in the binary for extraction by build tools.
    //!
    //! ## Usage
    //!
    //! ```rust
    //! use permissions::{static_permission, Permission, PermissionBuilder, PermissionKind, LocationPrecision};
    //!
    //! // Declare a camera permission (static / compile-time)
    //! const CAMERA: Permission = static_permission!(
    //!     Permission::new(PermissionKind::Camera, "Take photos")
    //! );
    //!
    //! // Declare a location permission with precision
    //! const LOCATION: Permission = static_permission!(
    //!     PermissionBuilder::location(LocationPrecision::Fine)
    //!         .with_description("Track your runs")
    //!         .build()
    //! );
    //!
    //! // Use the permission
    //! println!("Camera permission: {}", CAMERA.description());
    //! if let Some(android_perm) = CAMERA.android_permission() {
    //!     println!("Android permission: {}", android_perm);
    //! }
    //! ```
    //!
    //! > **Note:** `permission!` remains available as an alias for `static_permission!`
    //! > to preserve backward compatibility with existing code.

    pub use manganis_core::{
        AndroidArtifactMetadata, CustomPermissionBuilder, LocationPrecision, Permission,
        PermissionBuilder, PermissionKind, PermissionManifest, Platform, PlatformFlags,
        PlatformIdentifiers, SwiftPackageMetadata, SymbolData,
    };
    pub use manganis_macro::{permission, static_permission};

    /// Re-export macro helpers for use in generated code
    pub mod macro_helpers {
        pub use crate::macro_helpers::*;
    }
}

// FFI utilities and plugin metadata for Dioxus mobile platform APIs
//
// This crate provides common patterns and utilities for implementing
// mobile platform APIs in Dioxus applications. It handles the
// boilerplate for JNI (Android) and objc2 (iOS/macOS) bindings, build scripts,
// and platform-specific resource management.

#[cfg(any(target_os = "android", feature = "metadata"))]
mod android;

/// Darwin (iOS/macOS) platform utilities
#[doc(hidden)]
#[cfg(any(target_os = "ios", target_os = "macos", feature = "metadata"))]
pub mod darwin;

#[cfg(target_os = "android")]
pub use android::*;

// Export darwin module for iOS, macOS, and when metadata feature is enabled (for FFI macro)
#[cfg(any(target_os = "ios", target_os = "macos", feature = "metadata"))]
pub use darwin::*;

/// Re-export commonly used types for convenience
#[cfg(target_os = "android")]
pub use jni;

// Re-export objc2 for FFI macro generated code
#[cfg(any(target_os = "ios", target_os = "macos", feature = "metadata"))]
pub use objc2;

/// Re-export the android_plugin! macro when metadata feature is enabled
#[cfg(all(feature = "metadata", any(target_os = "android", feature = "metadata")))]
pub use manganis_macro::android_plugin;

/// Re-export the ios_plugin! macro when metadata feature is enabled
#[cfg(all(feature = "metadata", any(target_os = "ios", target_os = "macos")))]
pub use manganis_macro::ios_plugin;

/// Re-export the ffi attribute macro for native FFI bindings
/// This macro generates direct FFI bindings between Rust and native platforms (Swift/Kotlin)
pub use manganis_macro::ffi;

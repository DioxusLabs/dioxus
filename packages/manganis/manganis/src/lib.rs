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

    pub use permissions_core::{
        AndroidArtifactMetadata, CustomPermissionBuilder, LocationPrecision, Permission,
        PermissionBuilder, PermissionKind, PermissionManifest, Platform, PlatformFlags,
        PlatformIdentifiers, SwiftPackageMetadata, SymbolData,
    };
    pub use permissions_macro::{permission, static_permission};

    #[doc(hidden)]
    pub mod macro_helpers {
        //! Helper functions for macro expansion
        //!
        //! These functions are used internally by the `static_permission!()` macro (and its `permission!()` alias)
        //! and should not be used directly.

        // Re-export const_serialize types for convenience
        pub use const_serialize::{self, ConstStr, ConstVec, SerializeConst};
        // Re-export copy_bytes so generated code can use it without dx-macro-helpers dependency
        pub use dx_macro_helpers::copy_bytes;
        use permissions_core::{AndroidArtifactMetadata, SwiftPackageMetadata};
        pub use permissions_core::{Permission, SymbolData};

        const fn serialize_symbol_data(symbol_data: SymbolData) -> ConstVec<u8, 4096> {
            dx_macro_helpers::serialize_to_const_with_max_padded::<4096>(&symbol_data)
        }

        /// Serialize a permission into a const buffer (wrapped in `SymbolData::Permission`).
        pub const fn serialize_permission(permission: &Permission) -> ConstVec<u8, 4096> {
            serialize_symbol_data(SymbolData::Permission(*permission))
        }

        /// Serialize Android artifact metadata (wrapped in `SymbolData::AndroidArtifact`).
        pub const fn serialize_android_artifact(
            meta: &AndroidArtifactMetadata,
        ) -> ConstVec<u8, 4096> {
            serialize_symbol_data(SymbolData::AndroidArtifact(*meta))
        }

        /// Serialize Swift package metadata (wrapped in `SymbolData::SwiftPackage`).
        pub const fn serialize_swift_package(meta: &SwiftPackageMetadata) -> ConstVec<u8, 4096> {
            serialize_symbol_data(SymbolData::SwiftPackage(*meta))
        }
    }
}

// FFI utilities and plugin metadata for Dioxus mobile platform APIs
//
// This crate provides common patterns and utilities for implementing
// mobile platform APIs in Dioxus applications. It handles the
// boilerplate for JNI (Android) and objc2 (iOS/macOS) bindings, build scripts,
// and platform-specific resource management.

#[cfg(any(target_os = "android", feature = "metadata"))]
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
#[cfg(all(feature = "metadata", any(target_os = "android", feature = "metadata")))]
pub use manganis_macro::android_plugin;

/// Re-export the ios_plugin! macro when metadata feature is enabled
#[cfg(all(feature = "metadata", any(target_os = "ios", target_os = "macos")))]
pub use manganis_macro::ios_plugin;

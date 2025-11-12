#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

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
//! use permissions::{static_permission, Permission};
//!
//! // Declare a camera permission (static / compile-time)
//! const CAMERA: Permission = static_permission!(Camera, description = "Take photos");
//!
//! // Declare a location permission with precision
//! const LOCATION: Permission = static_permission!(Location(Fine), description = "Track your runs");
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
    LocationPrecision, Permission, PermissionKind, PermissionManifest, Platform, PlatformFlags,
    PlatformIdentifiers,
};
pub use permissions_macro::{permission, static_permission};

#[doc(hidden)]
pub mod macro_helpers {
    //! Helper functions for macro expansion
    //!
    //! These functions are used internally by the `static_permission!()` macro (and its `permission!()` alias)
    //! and should not be used directly.

    pub use const_serialize::{self, ConstStr, ConstVec, SerializeConst};
    pub use dx_macro_helpers::copy_bytes;
    pub use permissions_core::{Permission, SymbolData};

    /// Serialize a permission as SymbolData::Permission to a const buffer
    ///
    /// This wraps the permission in SymbolData::Permission variant for unified
    /// serialization with assets using the same __ASSETS__ prefix.
    ///
    /// Uses serialize_to_const which returns ConstVec<u8> (default size 1024).
    /// This should be sufficient for most permissions. CBOR serialization is
    /// self-describing, so padding doesn't affect deserialization.
    pub const fn serialize_permission(permission: &Permission) -> ConstVec<u8> {
        let symbol_data = SymbolData::Permission(*permission);
        // Use serialize_to_const which handles the const generic properly
        // It returns ConstVec<u8> (default 1024 size)
        dx_macro_helpers::serialize_to_const(&symbol_data, SymbolData::MEMORY_LAYOUT.size())
    }
}

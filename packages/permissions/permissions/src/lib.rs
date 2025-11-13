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
    CustomPermissionBuilder, LocationPrecision, Permission, PermissionBuilder, PermissionKind,
    PermissionManifest, Platform, PlatformFlags, PlatformIdentifiers,
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
    pub use permissions_core::{Permission, SymbolData};

    /// Serialize a permission as SymbolData::Permission to a const buffer
    ///
    /// This wraps the permission in SymbolData::Permission variant for unified
    /// serialization with assets using the same __ASSETS__ prefix.
    ///
    /// Uses a 4096-byte buffer to accommodate permissions with large ConstStr fields
    /// (especially custom permissions). The buffer is padded to the full buffer size (4096)
    /// to match the linker section size. const-serialize deserialization will ignore
    /// the padding (zeros) at the end.
    pub const fn serialize_permission(permission: &Permission) -> ConstVec<u8, 4096> {
        let symbol_data = SymbolData::Permission(*permission);
        // Serialize into a 4096-byte buffer and pad to full size
        // This matches the CLI's expectation for the fixed buffer size
        let mut data: ConstVec<u8, 4096> = ConstVec::new_with_max_size();
        data = const_serialize::serialize_const(&symbol_data, data);
        // Pad to full buffer size (4096) to match linker section size
        while data.len() < 4096 {
            data = data.push(0);
        }
        data
    }
}

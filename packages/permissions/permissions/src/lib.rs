#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

//! # Permissions
//!
//! A cross-platform permission management system with linker-based collection.
//!
//! This crate provides a unified API for declaring permissions across all platforms
//! (Android, iOS, macOS, Windows, Linux, Web) and embeds them in the binary for
//! extraction by build tools.
//!
//! ## Usage
//!
//! ```rust
//! use permissions::{permission, Permission};
//!
//! // Declare a camera permission
//! const CAMERA: Permission = permission!(Camera, description = "Take photos");
//!
//! // Declare a location permission with precision
//! const LOCATION: Permission = permission!(Location(Fine), description = "Track your runs");
//!
//! // Use the permission
//! println!("Camera permission: {}", CAMERA.description());
//! if let Some(android_perm) = CAMERA.android_permission() {
//!     println!("Android permission: {}", android_perm);
//! }
//! ```

pub use permissions_core::{
    LocationPrecision, Permission, PermissionKind, PermissionManifest, Platform, PlatformFlags,
    PlatformIdentifiers,
};
pub use permissions_macro::permission;

#[doc(hidden)]
pub mod macro_helpers {
    //! Helper functions for macro expansion
    //!
    //! These functions are used internally by the `permission!()` macro
    //! and should not be used directly.

    pub use const_serialize::{self, ConstStr, ConstVec, SerializeConst};
    pub use permissions_core::Permission;

    /// Serialize a permission to a const buffer with a large enough buffer size
    pub const fn serialize_permission(permission: &Permission) -> ConstVec<u8, 4096> {
        // First serialize with default buffer size
        let serialized = const_serialize::serialize_const(permission, ConstVec::new());
        // Then copy into a larger buffer and pad to MEMORY_LAYOUT size
        let mut data: ConstVec<u8, 4096> = ConstVec::new_with_max_size();
        data = data.extend(serialized.as_ref());
        // Reserve the maximum size of the permission
        while data.len() < Permission::MEMORY_LAYOUT.size() {
            data = data.push(0);
        }
        data
    }

    /// Copy a slice into a constant sized buffer at compile time
    pub const fn copy_bytes<const N: usize>(bytes: &[u8]) -> [u8; N] {
        let mut out = [0; N];
        let mut i = 0;
        while i < N {
            out[i] = bytes[i];
            i += 1;
        }
        out
    }
}

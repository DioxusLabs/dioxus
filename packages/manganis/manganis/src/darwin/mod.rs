//! Darwin (iOS/macOS) shared utilities for objc2-based APIs
//!
//! This module provides shared utilities for both iOS and macOS platforms
//! since they share the same Objective-C runtime and threading requirements
//! through objc2.

/// Re-export MainThreadMarker for convenience
pub use objc2::MainThreadMarker;

/// Macro helpers for FFI code generation
pub mod macro_helpers {
    pub use crate::macro_helpers::copy_bytes;
}

pub use manganis_core::SwiftPackageMetadata;

pub mod metadata {
    use manganis_core::SwiftPackageMetadata;

    /// Buffer type for serialized Swift metadata
    #[doc(hidden)]
    pub type SwiftMetadataBuffer = crate::macro_helpers::ConstVec<u8, 4096>;

    /// Serialize Swift package metadata for linker embedding
    #[doc(hidden)]
    pub const fn serialize_swift_metadata(meta: &SwiftPackageMetadata) -> SwiftMetadataBuffer {
        crate::macro_helpers::serialize_swift_package(meta)
    }
}

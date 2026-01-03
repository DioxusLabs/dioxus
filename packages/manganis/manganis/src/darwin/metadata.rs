//! Swift package metadata wrappers for linker-based collection.

pub use manganis_core::SwiftPackageMetadata as SwiftSourceMetadata;

/// Buffer type for serialized Swift metadata
#[doc(hidden)]
pub type SwiftMetadataBuffer = crate::macro_helpers::ConstVec<u8, 4096>;

/// Serialize Swift package metadata for linker embedding
#[doc(hidden)]
pub const fn serialize_swift_metadata(meta: &SwiftSourceMetadata) -> SwiftMetadataBuffer {
    crate::macro_helpers::serialize_swift_package(meta)
}

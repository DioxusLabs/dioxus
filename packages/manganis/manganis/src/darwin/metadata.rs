//! Swift package metadata wrappers for linker-based collection.

#[cfg(feature = "metadata")]
pub use permissions::SwiftPackageMetadata as SwiftSourceMetadata;

#[cfg(feature = "metadata")]
pub type SwiftMetadataBuffer = permissions::macro_helpers::ConstVec<u8, 4096>;

#[cfg(feature = "metadata")]
pub const fn serialize_swift_metadata(meta: &SwiftSourceMetadata) -> SwiftMetadataBuffer {
    permissions::macro_helpers::serialize_swift_package(meta)
}

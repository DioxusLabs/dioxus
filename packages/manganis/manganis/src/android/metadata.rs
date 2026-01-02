//! Android metadata wrappers for linker-based collection.

#[cfg(feature = "metadata")]
pub use permissions::AndroidArtifactMetadata;

#[cfg(feature = "metadata")]
pub type AndroidMetadataBuffer = permissions::macro_helpers::ConstVec<u8, 4096>;

#[cfg(feature = "metadata")]
pub const fn serialize_android_metadata(meta: &AndroidArtifactMetadata) -> AndroidMetadataBuffer {
    permissions::macro_helpers::serialize_android_artifact(meta)
}

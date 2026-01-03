//! Android metadata wrappers for linker-based collection.

pub use manganis_core::AndroidArtifactMetadata;

pub type AndroidMetadataBuffer = crate::macro_helpers::ConstVec<u8, 4096>;

pub const fn serialize_android_metadata(meta: &AndroidArtifactMetadata) -> AndroidMetadataBuffer {
    crate::macro_helpers::serialize_android_artifact(meta)
}

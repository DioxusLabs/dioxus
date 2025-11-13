//! Android metadata types for linker-based collection

#[cfg(feature = "metadata")]
use const_serialize::{ConstStr, ConstVec, SerializeConst};

/// Android artifact metadata that can be embedded in the binary.
///
/// This struct contains information about prebuilt Android artifacts (e.g. AARs)
/// that should be linked into the final Gradle project. The data is embedded via
/// linker sections similar to how permissions and Swift metadata are handled.
#[cfg(feature = "metadata")]
#[derive(Debug, Clone, PartialEq, Eq, SerializeConst)]
pub struct AndroidArtifactMetadata {
    pub plugin_name: ConstStr,
    pub artifact_path: ConstStr,
    pub gradle_dependencies: ConstStr,
}

#[cfg(feature = "metadata")]
impl AndroidArtifactMetadata {
    pub const fn new(
        plugin_name: &'static str,
        artifact_path: &'static str,
        gradle_dependencies: &'static str,
    ) -> Self {
        Self {
            plugin_name: ConstStr::new(plugin_name),
            artifact_path: ConstStr::new(artifact_path),
            gradle_dependencies: ConstStr::new(gradle_dependencies),
        }
    }

    pub const SERIALIZED_SIZE: usize = 4096;
}

#[cfg(feature = "metadata")]
pub type AndroidMetadataBuffer = ConstVec<u8, { AndroidArtifactMetadata::SERIALIZED_SIZE }>;

#[cfg(feature = "metadata")]
pub const fn serialize_android_metadata(meta: &AndroidArtifactMetadata) -> AndroidMetadataBuffer {
    let serialized = const_serialize::serialize_const(meta, ConstVec::new());
    let mut buffer: AndroidMetadataBuffer = ConstVec::new_with_max_size();
    buffer = buffer.extend(serialized.as_ref());
    while buffer.len() < AndroidArtifactMetadata::SERIALIZED_SIZE {
        buffer = buffer.push(0);
    }
    buffer
}

//! Darwin (iOS/macOS) metadata types for linker-based collection
//!
//! This module provides metadata types for Swift source files that need to be
//! bundled into iOS/macOS apps, similar to how Java/Kotlin files work for Android.

#[cfg(feature = "metadata")]
use const_serialize::{ConstStr, ConstVec, SerializeConst};

/// Swift Package metadata embedded in the final binary.
#[cfg(feature = "metadata")]
#[derive(Debug, Clone, PartialEq, Eq, SerializeConst)]
pub struct SwiftSourceMetadata {
    /// Plugin identifier (e.g. "geolocation")
    pub plugin_name: ConstStr,
    /// Absolute path to the Swift package declared by the plugin
    pub package_path: ConstStr,
    /// Swift product to link from that package
    pub product: ConstStr,
}

#[cfg(feature = "metadata")]
impl SwiftSourceMetadata {
    /// Create metadata for a Swift package declaration.
    pub const fn new(
        plugin_name: &'static str,
        package_path: &'static str,
        product: &'static str,
    ) -> Self {
        Self {
            plugin_name: ConstStr::new(plugin_name),
            package_path: ConstStr::new(package_path),
            product: ConstStr::new(product),
        }
    }

    /// The size of the serialized data buffer
    pub const SERIALIZED_SIZE: usize = 4096;
}

/// Buffer type used for serialized Swift metadata blobs
#[cfg(feature = "metadata")]
pub type SwiftMetadataBuffer = ConstVec<u8, { SwiftSourceMetadata::SERIALIZED_SIZE }>;

/// Serialize metadata into a fixed-size buffer for linker embedding
#[cfg(feature = "metadata")]
pub const fn serialize_swift_metadata(meta: &SwiftSourceMetadata) -> SwiftMetadataBuffer {
    let serialized = const_serialize::serialize_const(meta, ConstVec::new());
    let mut buffer: SwiftMetadataBuffer = ConstVec::new_with_max_size();
    buffer = buffer.extend(serialized.as_ref());
    // Pad to the expected size to ensure consistent linker symbols
    while buffer.len() < SwiftSourceMetadata::SERIALIZED_SIZE {
        buffer = buffer.push(0);
    }
    buffer
}

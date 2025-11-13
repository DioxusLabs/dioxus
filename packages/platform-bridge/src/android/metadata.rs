//! Android metadata types for linker-based collection

#[cfg(feature = "metadata")]
use const_serialize::{ConstStr, ConstVec, SerializeConst};

/// Java source file metadata that can be embedded in the binary
///
/// This struct contains information about Java source files that need to be
/// compiled into the Android APK. It uses const-serialize to be embeddable
/// in linker sections, similar to how permissions work.
#[cfg(feature = "metadata")]
#[derive(Debug, Clone, PartialEq, Eq, SerializeConst)]
pub struct JavaSourceMetadata {
    /// Java package name (e.g. "dioxus.mobile.geolocation")
    pub package_name: ConstStr,
    /// Plugin identifier for organization (e.g. "geolocation")
    pub plugin_name: ConstStr,
    /// Number of files
    pub file_count: u8,
    /// File paths - absolute paths to Java source files
    /// Example: "/path/to/crate/src/sys/android/LocationCallback.java"
    /// Maximum 8 files supported
    pub files: [ConstStr; 8],
}

#[cfg(feature = "metadata")]
impl JavaSourceMetadata {
    /// Create new Java source metadata with absolute file paths
    ///
    /// Takes full absolute paths to Java source files. The paths are embedded at compile time
    /// using the `android_plugin!()` macro, which uses `env!("CARGO_MANIFEST_DIR")` to resolve
    /// paths relative to the calling crate.
    ///
    /// # Example
    /// ```rust,no_run
    /// JavaSourceMetadata::new(
    ///     "dioxus.mobile.geolocation",
    ///     "geolocation",
    ///     &[
    ///         "/path/to/crate/src/sys/android/LocationCallback.java",
    ///         "/path/to/crate/src/sys/android/PermissionsHelper.java",
    ///     ],
    /// )
    /// ```
    pub const fn new(
        package_name: &'static str,
        plugin_name: &'static str,
        file_paths: &'static [&'static str],
    ) -> Self {
        let mut file_array = [ConstStr::new(""); 8];
        let mut i = 0;
        while i < file_paths.len() && i < 8 {
            file_array[i] = ConstStr::new(file_paths[i]);
            i += 1;
        }

        Self {
            package_name: ConstStr::new(package_name),
            plugin_name: ConstStr::new(plugin_name),
            file_count: file_paths.len() as u8,
            files: file_array,
        }
    }

    /// The size of the serialized data buffer
    pub const SERIALIZED_SIZE: usize = 4096;
}

/// Buffer type used for serialized Java metadata blobs
#[cfg(feature = "metadata")]
pub type JavaMetadataBuffer = ConstVec<u8, { JavaSourceMetadata::SERIALIZED_SIZE }>;

/// Serialize metadata into a fixed-size buffer for linker embedding
#[cfg(feature = "metadata")]
pub const fn serialize_java_metadata(meta: &JavaSourceMetadata) -> JavaMetadataBuffer {
    let serialized = const_serialize::serialize_const(meta, ConstVec::new());
    let mut buffer: JavaMetadataBuffer = ConstVec::new_with_max_size();
    buffer = buffer.extend(serialized.as_ref());
    // Pad to the expected size to ensure consistent linker symbols
    while buffer.len() < JavaSourceMetadata::SERIALIZED_SIZE {
        buffer = buffer.push(0);
    }
    buffer
}

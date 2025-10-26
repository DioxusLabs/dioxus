//! Android metadata types for linker-based collection

#[cfg(feature = "metadata")]
use const_serialize::{ConstStr, SerializeConst};

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
    /// File paths - just filenames, not full paths (max 8 files)
    pub files: [ConstStr; 8],
}

#[cfg(feature = "metadata")]
impl JavaSourceMetadata {
    /// Create new Java source metadata
    pub const fn new(
        package_name: &'static str,
        plugin_name: &'static str,
        files: &'static [&'static str],
    ) -> Self {
        let mut file_array = [ConstStr::new(""); 8];
        let mut i = 0;
        while i < files.len() && i < 8 {
            file_array[i] = ConstStr::new(files[i]);
            i += 1;
        }

        Self {
            package_name: ConstStr::new(package_name),
            plugin_name: ConstStr::new(plugin_name),
            file_count: files.len() as u8,
            files: file_array,
        }
    }
    /// The size of the serialized data buffer
    pub const SERIALIZED_SIZE: usize = 4096;
}

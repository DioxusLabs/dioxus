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
    /// File paths - can be either:
    /// - Just filenames (legacy): "LocationCallback.java"
    /// - Absolute paths (new): "/path/to/crate/src/sys/android/LocationCallback.java"
    /// Maximum 8 files supported
    pub files: [ConstStr; 8],
}

#[cfg(feature = "metadata")]
impl JavaSourceMetadata {
    /// Create new Java source metadata with filenames only (legacy)
    ///
    /// The filenames are relative to the crate's src/sys/android/ or src/android/ directory.
    /// At build time, the CLI will search the workspace to find the actual files.
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

    /// Create new Java source metadata with absolute file paths (new)
    ///
    /// Takes full absolute paths to Java source files. This allows the CLI to
    /// directly access files without searching the workspace, improving build performance.
    /// 
    /// # Example
    /// ```rust,no_run
    /// JavaSourceMetadata::new_with_paths(
    ///     "dioxus.mobile.geolocation",
    ///     "geolocation",
    ///     &[
    ///         "/path/to/crate/src/sys/android/LocationCallback.java",
    ///         "/path/to/crate/src/sys/android/PermissionsHelper.java",
    ///     ],
    /// )
    /// ```
    pub const fn new_with_paths(
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

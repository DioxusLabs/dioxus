//! Android Java source collection from compiled binaries
//!
//! This module extracts Java source metadata from embedded linker symbols
//! using the unified `__MANGANIS__` prefix with `LinkerSymbol::JavaSource`.
//! The metadata is used by the Gradle build process to compile Java sources to DEX.

/// Metadata about Java sources that need to be compiled to DEX
/// This mirrors the struct from platform-bridge
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaSourceMetadata {
    /// File paths relative to crate root
    pub files: Vec<String>,
    /// Java package name (e.g. "dioxus.mobile.geolocation")
    pub package_name: String,
    /// Plugin identifier for organization (e.g. "geolocation")
    pub plugin_name: String,
}

impl JavaSourceMetadata {
    /// Create from platform-bridge::android::JavaSourceMetadata
    fn from_platform_bridge(java_source: platform_bridge::android::JavaSourceMetadata) -> Self {
        Self {
            package_name: java_source.package_name.as_str().to_string(),
            plugin_name: java_source.plugin_name.as_str().to_string(),
            files: java_source.files[..java_source.file_count as usize]
                .iter()
                .map(|s| s.as_str().to_string())
                .collect(),
        }
    }
}

/// A manifest of all Java sources found in a binary
#[derive(Debug, Clone, Default)]
pub struct JavaSourceManifest {
    sources: Vec<JavaSourceMetadata>,
}

impl JavaSourceManifest {
    pub fn new(sources: Vec<JavaSourceMetadata>) -> Self {
        Self { sources }
    }

    pub fn sources(&self) -> &[JavaSourceMetadata] {
        &self.sources
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

/// Extract all Java sources from the given file.
///
/// This function extracts Java sources from the unified __MANGANIS__ symbols
/// by calling the asset extraction function which handles LinkerSymbol enum.
pub(crate) async fn extract_java_sources_from_file(path: impl AsRef<Path>) -> Result<JavaSourceManifest> {
    use super::assets;
    
    // Extract Java sources from unified symbol collection
    let (_assets, _permissions, java_sources) = assets::extract_assets_from_file(path).await?;
    
    // Convert platform-bridge::android::JavaSourceMetadata to JavaSourceMetadata
    let mut sources = Vec::new();
    for java_source in java_sources {
        sources.push(JavaSourceMetadata::from_platform_bridge(java_source));
    }
    
    Ok(JavaSourceManifest::new(sources))
}



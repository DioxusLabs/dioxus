//! Android Java source collection from compiled binaries
//!
//! This module extracts Java source metadata from embedded linker symbols,
//! similar to how permissions and manganis work. It finds `__JAVA_SOURCE__`
//! symbols in the binary and deserializes them into metadata that can be
//! used by the Gradle build process.

use std::io::Read;
use std::path::Path;

use crate::Result;

const JAVA_SOURCE_SYMBOL_PREFIX: &str = "__JAVA_SOURCE__";

use super::linker_symbols;

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
    /// Create from the platform-bridge SerializeConst version
    fn from_const_serialize(
        package_name: const_serialize::ConstStr,
        plugin_name: const_serialize::ConstStr,
        file_count: u8,
        files: [const_serialize::ConstStr; 8],
    ) -> Self {
        Self {
            package_name: package_name.as_str().to_string(),
            plugin_name: plugin_name.as_str().to_string(),
            files: files[..file_count as usize]
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

/// Extract all Java sources from the given file
pub(crate) fn extract_java_sources_from_file(path: impl AsRef<Path>) -> Result<JavaSourceManifest> {
    let path = path.as_ref();
    let offsets = linker_symbols::find_symbol_offsets_from_path(path, JAVA_SOURCE_SYMBOL_PREFIX)?;

    let mut file = std::fs::File::open(path)?;
    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents)?;

    let mut sources = Vec::new();

    // Parse the metadata from each symbol offset
    // The format is: (package_name: &str, plugin_name: &str, files: &[&str])
    for offset in offsets {
        match parse_java_metadata_at_offset(&file_contents, offset as usize) {
            Ok(metadata) => {
                tracing::debug!(
                    "Extracted Java metadata: plugin={}, package={}, files={:?}",
                    metadata.plugin_name,
                    metadata.package_name,
                    metadata.files
                );
                sources.push(metadata);
            }
            Err(e) => {
                tracing::warn!("Failed to parse Java metadata at offset {}: {}", offset, e);
            }
        }
    }

    if !sources.is_empty() {
        tracing::info!(
            "Extracted {} Java source declarations from binary",
            sources.len()
        );
    }

    Ok(JavaSourceManifest::new(sources))
}

/// Parse Java metadata from binary data at the given offset
///
/// The data is serialized using const-serialize and contains:
/// - package_name: ConstStr
/// - plugin_name: ConstStr  
/// - file_count: u8
/// - files: [ConstStr; 8]
fn parse_java_metadata_at_offset(data: &[u8], offset: usize) -> Result<JavaSourceMetadata> {
    use const_serialize::ConstStr;

    // Read the serialized data (padded to 4096 bytes like permissions)
    let end = (offset + 4096).min(data.len());
    let metadata_bytes = &data[offset..end];

    let buffer = const_serialize::ConstReadBuffer::new(metadata_bytes);

    // Deserialize the struct fields
    // The SerializeConst derive creates a tuple-like serialization
    if let Some((buffer, package_name)) = const_serialize::deserialize_const!(ConstStr, buffer) {
        if let Some((buffer, plugin_name)) = const_serialize::deserialize_const!(ConstStr, buffer) {
            if let Some((buffer, file_count)) = const_serialize::deserialize_const!(u8, buffer) {
                if let Some((_, files)) = const_serialize::deserialize_const!([ConstStr; 8], buffer)
                {
                    return Ok(JavaSourceMetadata::from_const_serialize(
                        package_name,
                        plugin_name,
                        file_count,
                        files,
                    ));
                }
            }
        }
    }

    anyhow::bail!("Failed to deserialize Java metadata at offset {}", offset)
}

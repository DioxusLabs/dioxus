//! iOS Swift source collection from compiled binaries
//!
//! This module extracts Swift source metadata from embedded linker symbols,
//! similar to how permissions and Java sources work. It finds `__SWIFT_SOURCE__`
//! symbols in the binary and deserializes them into metadata that can be
//! used by the iOS/macOS build process.

use std::io::Read;
use std::path::Path;

use crate::Result;
use anyhow::Context;

const SWIFT_SOURCE_SYMBOL_PREFIX: &str = "__SWIFT_SOURCE__";

use super::linker_symbols;

/// Metadata about Swift packages that need to be linked into the iOS/macOS app bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwiftSourceMetadata {
    /// Plugin identifier for organization (e.g. "geolocation")
    plugin_name: String,
    package_path: String,
    product: String,
}

impl SwiftSourceMetadata {
    fn from_platform_metadata(meta: dioxus_platform_bridge::darwin::SwiftSourceMetadata) -> Self {
        Self {
            plugin_name: meta.plugin_name.as_str().to_string(),
            package_path: meta.package_path.as_str().to_string(),
            product: meta.product.as_str().to_string(),
        }
    }

    pub fn plugin_name(&self) -> &str {
        &self.plugin_name
    }

    pub fn package_path(&self) -> &str {
        &self.package_path
    }

    pub fn product(&self) -> &str {
        &self.product
    }
}

/// A manifest of all Swift sources found in a binary
#[derive(Debug, Clone, Default)]
pub struct SwiftSourceManifest {
    sources: Vec<SwiftSourceMetadata>,
}

impl SwiftSourceManifest {
    pub fn new(sources: Vec<SwiftSourceMetadata>) -> Self {
        Self { sources }
    }

    pub fn sources(&self) -> &[SwiftSourceMetadata] {
        &self.sources
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

/// Extract all Swift sources from the given file
pub(crate) fn extract_swift_sources_from_file(
    path: impl AsRef<Path>,
) -> Result<SwiftSourceManifest> {
    let path = path.as_ref();
    let offsets = linker_symbols::find_symbol_offsets_from_path(path, SWIFT_SOURCE_SYMBOL_PREFIX)?;

    let mut file = std::fs::File::open(path)?;
    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents)?;

    let mut sources = Vec::new();

    for offset in offsets {
        let metadata = parse_swift_metadata_at_offset(&file_contents, offset as usize)
            .with_context(|| {
                format!(
                    "Failed to parse Swift metadata embedded in binary (offset {})",
                    offset
                )
            })?;

        tracing::debug!(
            "Extracted Swift metadata: plugin={} package={} product={}",
            metadata.plugin_name(),
            metadata.package_path(),
            metadata.product()
        );
        sources.push(metadata);
    }

    if !sources.is_empty() {
        tracing::info!(
            "Extracted {} Swift source declarations from binary",
            sources.len()
        );
    }

    Ok(SwiftSourceManifest::new(sources))
}

/// Parse Swift metadata from binary data at the given offset.
fn parse_swift_metadata_at_offset(data: &[u8], offset: usize) -> Result<SwiftSourceMetadata> {
    // Read the serialized data (padded to 4096 bytes like permissions)
    let end = (offset + 4096).min(data.len());
    let metadata_bytes = &data[offset..end];

    if let Some((_, platform_meta)) = const_serialize::deserialize_const!(
        dioxus_platform_bridge::darwin::SwiftSourceMetadata,
        metadata_bytes
    ) {
        return Ok(SwiftSourceMetadata::from_platform_metadata(platform_meta));
    }

    anyhow::bail!("Failed to deserialize Swift metadata at offset {}", offset)
}

//! Android artifact collection from compiled binaries.
//!
//! This module extracts Android artifact metadata (AAR paths) from embedded linker symbols,
//! similar to how permissions and Swift sources are discovered. It finds
//! `__ANDROID_ARTIFACT__` symbols in the binary and deserializes them so the
//! Gradle build can consume the prebuilt plugins.

use std::io::Read;
use std::path::Path;

use crate::Result;

const ANDROID_ARTIFACT_SYMBOL_PREFIX: &str = "__ANDROID_ARTIFACT__";

use super::linker_symbols;

/// Metadata about Android artifacts (AARs) that should be included in the Gradle build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidArtifactMetadata {
    pub plugin_name: String,
    pub artifact_path: String,
    pub gradle_dependencies: Vec<String>,
}

impl AndroidArtifactMetadata {
    fn from_const(meta: dioxus_platform_bridge::android::AndroidArtifactMetadata) -> Self {
        let deps = meta
            .gradle_dependencies
            .as_str()
            .split('\n')
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect();
        Self {
            plugin_name: meta.plugin_name.as_str().to_string(),
            artifact_path: meta.artifact_path.as_str().to_string(),
            gradle_dependencies: deps,
        }
    }
}

/// Manifest of all Android artifacts found in a binary.
#[derive(Debug, Clone, Default)]
pub struct AndroidArtifactManifest {
    artifacts: Vec<AndroidArtifactMetadata>,
}

impl AndroidArtifactManifest {
    pub fn new(artifacts: Vec<AndroidArtifactMetadata>) -> Self {
        Self { artifacts }
    }

    pub fn artifacts(&self) -> &[AndroidArtifactMetadata] {
        &self.artifacts
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

/// Extract all Android artifacts from the given file.
pub(crate) fn extract_android_artifacts_from_file(
    path: impl AsRef<Path>,
) -> Result<AndroidArtifactManifest> {
    let path = path.as_ref();
    let offsets =
        linker_symbols::find_symbol_offsets_from_path(path, ANDROID_ARTIFACT_SYMBOL_PREFIX)?;

    let mut file = std::fs::File::open(path)?;
    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents)?;

    let mut artifacts = Vec::new();
    for offset in offsets {
        match parse_android_metadata_at_offset(&file_contents, offset as usize) {
            Ok(metadata) => {
                tracing::debug!(
                    "Extracted Android artifact metadata: plugin={} path={} deps={}",
                    metadata.plugin_name,
                    metadata.artifact_path,
                    metadata.gradle_dependencies.len()
                );
                artifacts.push(metadata);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse Android metadata at offset {}: {}",
                    offset,
                    e
                );
            }
        }
    }

    if !artifacts.is_empty() {
        tracing::info!(
            "Extracted {} Android artifact declaration(s) from binary",
            artifacts.len()
        );
    }

    Ok(AndroidArtifactManifest::new(artifacts))
}

fn parse_android_metadata_at_offset(data: &[u8], offset: usize) -> Result<AndroidArtifactMetadata> {
    let end = (offset + 4096).min(data.len());
    let metadata_bytes = &data[offset..end];

    if let Some((_, meta)) = const_serialize::deserialize_const!(
        dioxus_platform_bridge::android::AndroidArtifactMetadata,
        metadata_bytes
    ) {
        return Ok(AndroidArtifactMetadata::from_const(meta));
    }

    anyhow::bail!(
        "Failed to deserialize Android metadata at offset {}",
        offset
    )
}

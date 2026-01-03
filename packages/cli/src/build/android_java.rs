//! Android artifact manifest helpers.

use manganis_core::AndroidArtifactMetadata;

/// Manifest of all Android artifacts declared by dependencies.
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

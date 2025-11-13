//! iOS/macOS Swift package manifest helpers.

use permissions::SwiftPackageMetadata as SwiftSourceMetadata;

/// Manifest of Swift packages embedded in the binary.
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

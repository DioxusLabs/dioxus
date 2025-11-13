use const_serialize::{ConstStr, SerializeConst};
use manganis_core::BundledAsset;

use crate::Permission;

/// Unified symbol data that can represent both assets and permissions
///
/// This enum is used to serialize different types of metadata into the binary
/// using the same `__ASSETS__` symbol prefix. The CBOR format allows for
/// self-describing data, making it easy to add new variants in the future.
///
/// Variant order does NOT matter for CBOR enum serialization - variants are
/// matched by name (string), not by position or tag value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeConst)]
#[repr(C, u8)]
pub enum SymbolData {
    /// An asset that should be bundled with the application
    Asset(BundledAsset),
    /// A permission declaration for the application
    Permission(Permission),
    /// Android plugin metadata (prebuilt artifacts + Gradle deps)
    AndroidArtifact(AndroidArtifactMetadata),
    /// Swift package metadata (SPM location + product)
    SwiftPackage(SwiftPackageMetadata),
}

/// Metadata describing an Android plugin artifact (.aar) that must be copied into the host Gradle project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeConst)]
pub struct AndroidArtifactMetadata {
    pub plugin_name: ConstStr,
    pub artifact_path: ConstStr,
    pub gradle_dependencies: ConstStr,
}

impl AndroidArtifactMetadata {
    pub const fn new(
        plugin_name: &'static str,
        artifact_path: &'static str,
        gradle_dependencies: &'static str,
    ) -> Self {
        Self {
            plugin_name: ConstStr::new(plugin_name),
            artifact_path: ConstStr::new(artifact_path),
            gradle_dependencies: ConstStr::new(gradle_dependencies),
        }
    }
}

/// Metadata for a Swift package that needs to be linked into the app (iOS/macOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeConst)]
pub struct SwiftPackageMetadata {
    pub plugin_name: ConstStr,
    pub package_path: ConstStr,
    pub product: ConstStr,
}

impl SwiftPackageMetadata {
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
}

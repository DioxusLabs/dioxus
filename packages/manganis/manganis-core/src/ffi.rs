use crate::BundledAsset;
use const_serialize::{ConstStr, SerializeConst};
use const_serialize_08 as const_serialize;
use std::hash::{Hash, Hasher};

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
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
pub enum SymbolData {
    /// An asset that should be bundled with the application
    Asset(BundledAsset),

    /// Android plugin metadata (prebuilt artifacts + Gradle deps)
    AndroidArtifact(AndroidArtifactMetadata),

    /// Swift package metadata (SPM location + product)
    SwiftPackage(SwiftPackageMetadata),
}

/// Platform categories for permission mapping
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeConst)]
pub enum Platform {
    Android,
    Ios,
    Macos,
}

/// Bit flags for supported platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeConst)]
pub struct PlatformFlags(u8);

impl PlatformFlags {
    pub const fn new() -> Self {
        Self(0)
    }
}

impl Default for PlatformFlags {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformFlags {
    pub const fn with_platform(mut self, platform: Platform) -> Self {
        self.0 |= 1 << platform as u8;
        self
    }

    pub const fn supports(&self, platform: Platform) -> bool {
        (self.0 & (1 << platform as u8)) != 0
    }

    pub const fn all() -> Self {
        Self(0b000111) // Android + iOS + macOS
    }

    pub const fn mobile() -> Self {
        Self(0b000011) // Android + iOS
    }
}

/// Platform-specific permission identifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlatformIdentifiers {
    pub android: Option<ConstStr>,
    pub ios: Option<ConstStr>,
    pub macos: Option<ConstStr>,
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

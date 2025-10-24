use const_serialize::{ConstStr, SerializeConst};
use std::hash::{Hash, Hasher};

use crate::{PermissionKind, Platform, PlatformFlags, PlatformIdentifiers};

/// A permission declaration that can be embedded in the binary
///
/// This struct contains all the information needed to declare a permission
/// across all supported platforms. It uses const-serialize to be embeddable
/// in linker sections.
#[derive(Debug, Clone, PartialEq, Eq, SerializeConst)]
pub struct Permission {
    /// The kind of permission being declared
    kind: PermissionKind,
    /// User-facing description of why this permission is needed
    description: ConstStr,
    /// Platforms where this permission is supported
    supported_platforms: PlatformFlags,
}

impl Permission {
    /// Create a new permission with the given kind and description
    pub const fn new(kind: PermissionKind, description: &'static str) -> Self {
        let supported_platforms = kind.supported_platforms();
        Self {
            kind,
            description: ConstStr::new(description),
            supported_platforms,
        }
    }

    /// Get the permission kind
    pub const fn kind(&self) -> &PermissionKind {
        &self.kind
    }

    /// Get the user-facing description
    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    /// Get the platforms that support this permission
    pub const fn supported_platforms(&self) -> PlatformFlags {
        self.supported_platforms
    }

    /// Check if this permission is supported on the given platform
    pub const fn supports_platform(&self, platform: Platform) -> bool {
        self.supported_platforms.supports(platform)
    }

    /// Get the platform-specific identifiers for this permission
    pub const fn platform_identifiers(&self) -> PlatformIdentifiers {
        self.kind.platform_identifiers()
    }

    /// Get the Android permission string, if supported
    pub fn android_permission(&self) -> Option<String> {
        self.platform_identifiers()
            .android
            .map(|s| s.as_str().to_string())
    }

    /// Get the iOS/macOS usage description key, if supported
    pub fn ios_key(&self) -> Option<String> {
        self.platform_identifiers()
            .ios
            .map(|s| s.as_str().to_string())
    }

    /// Get the macOS usage description key, if supported
    pub fn macos_key(&self) -> Option<String> {
        self.platform_identifiers()
            .macos
            .map(|s| s.as_str().to_string())
    }

    /// Get the Windows capability string, if supported
    pub fn windows_capability(&self) -> Option<String> {
        self.platform_identifiers()
            .windows
            .map(|s| s.as_str().to_string())
    }

    /// Get the Linux permission string, if supported
    pub fn linux_permission(&self) -> Option<String> {
        self.platform_identifiers()
            .linux
            .map(|s| s.as_str().to_string())
    }

    /// Get the Web API permission string, if supported
    pub fn web_permission(&self) -> Option<String> {
        self.platform_identifiers()
            .web
            .map(|s| s.as_str().to_string())
    }

    /// Create a permission from embedded data (used by the macro)
    ///
    /// This function is used internally by the macro to create a Permission
    /// from data embedded in the binary via linker sections.
    pub const fn from_embedded() -> Self {
        // This is a placeholder implementation. The actual deserialization
        // will be handled by the macro expansion.
        Self {
            kind: PermissionKind::Camera,   // Placeholder
            description: ConstStr::new(""), // Placeholder
            supported_platforms: PlatformFlags::new(),
        }
    }
}

impl Hash for Permission {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.description.hash(state);
        self.supported_platforms.hash(state);
    }
}

/// A collection of permissions that can be serialized and embedded
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionManifest {
    /// All permissions declared in the application
    permissions: Vec<Permission>,
}

impl PermissionManifest {
    /// Create a new empty permission manifest
    pub fn new() -> Self {
        Self {
            permissions: Vec::new(),
        }
    }

    /// Add a permission to the manifest
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.push(permission);
    }

    /// Get all permissions in the manifest
    pub fn permissions(&self) -> &[Permission] {
        &self.permissions
    }

    /// Get permissions for a specific platform
    pub fn permissions_for_platform(&self, platform: Platform) -> Vec<&Permission> {
        self.permissions
            .iter()
            .filter(|p| p.supports_platform(platform))
            .collect()
    }

    /// Check if the manifest contains any permissions
    pub fn is_empty(&self) -> bool {
        self.permissions.is_empty()
    }

    /// Get the number of permissions in the manifest
    pub fn len(&self) -> usize {
        self.permissions.len()
    }
}

impl Default for PermissionManifest {
    fn default() -> Self {
        Self::new()
    }
}

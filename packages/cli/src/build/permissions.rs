//! The dioxus permission system.
//!
//! This module extracts permissions from compiled binaries and generates platform-specific
//! manifest files for platforms that require build-time permission declarations.
//!
//! Platforms requiring build-time manifests:
//! - Android: AndroidManifest.xml with `uses-permission` declarations
//! - iOS/macOS: Info.plist with usage description keys
//!
//! Other platforms (Linux, Web, Windows desktop) use runtime-only permissions
//! and do not require build-time manifest generation.
use manganis_core::{Permission, Platform};
use serde::Serialize;

/// A collection of permissions that can be serialized and embedded
#[derive(Default, Debug, Clone, PartialEq, Eq)]
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

    /// Create a manifest from an existing list of permissions
    pub fn from_permissions(permissions: Vec<Permission>) -> Self {
        Self { permissions }
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

/// Android permission for Handlebars template
#[derive(Debug, Clone, Serialize)]
pub struct AndroidPermission {
    pub name: String,
    pub description: String,
}

/// iOS permission for Handlebars template
#[derive(Debug, Clone, Serialize)]
pub struct IosPermission {
    pub key: String,
    pub description: String,
}

/// macOS permission for Handlebars template
#[derive(Debug, Clone, Serialize)]
pub struct MacosPermission {
    pub key: String,
    pub description: String,
}

/// Get Android permissions for Handlebars template
pub(crate) fn get_android_permissions(manifest: &PermissionManifest) -> Vec<AndroidPermission> {
    manifest
        .permissions_for_platform(Platform::Android)
        .iter()
        .filter_map(|perm| {
            perm.android_permission()
                .map(|android_perm| AndroidPermission {
                    name: android_perm.to_string(),
                    description: perm.description().to_string(),
                })
        })
        .collect()
}

/// Get iOS permissions for Handlebars template
pub(crate) fn get_ios_permissions(manifest: &PermissionManifest) -> Vec<IosPermission> {
    manifest
        .permissions_for_platform(Platform::Ios)
        .iter()
        .filter_map(|perm| {
            perm.ios_key().map(|key| IosPermission {
                key: key.to_string(),
                description: perm.description().to_string(),
            })
        })
        .collect()
}

/// Get macOS permissions for Handlebars template
pub(crate) fn get_macos_permissions(manifest: &PermissionManifest) -> Vec<MacosPermission> {
    manifest
        .permissions_for_platform(Platform::Macos)
        .iter()
        .filter_map(|perm| {
            perm.macos_key().map(|key| MacosPermission {
                key: key.to_string(),
                description: perm.description().to_string(),
            })
        })
        .collect()
}

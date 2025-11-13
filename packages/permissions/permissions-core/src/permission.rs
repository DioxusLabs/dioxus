use const_serialize::{ConstStr, SerializeConst};
use std::hash::{Hash, Hasher};

use crate::{PermissionKind, Platform, PlatformFlags, PlatformIdentifiers};

/// A permission declaration that can be embedded in the binary
///
/// This struct contains all the information needed to declare a permission
/// across all supported platforms. It uses const-serialize to be embeddable
/// in linker sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeConst)]
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

impl Default for PermissionManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for custom permissions with platform-specific identifiers
///
/// This builder uses named methods to specify platform identifiers,
/// making it clear which value belongs to which platform.
///
/// # Examples
///
/// ```rust
/// use permissions_core::{Permission, PermissionBuilder};
///
/// const CUSTOM: Permission = PermissionBuilder::custom()
///     .with_android("android.permission.MY_PERMISSION")
///     .with_ios("NSMyUsageDescription")
///     .with_macos("NSMyUsageDescription")
///     .with_description("Custom permission")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct CustomPermissionBuilder {
    android: Option<ConstStr>,
    ios: Option<ConstStr>,
    macos: Option<ConstStr>,
    description: Option<ConstStr>,
}

impl CustomPermissionBuilder {
    /// Set the Android permission string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use permissions_core::{Permission, PermissionBuilder};
    ///
    /// const PERM: Permission = PermissionBuilder::custom()
    ///     .with_android("android.permission.READ_EXTERNAL_STORAGE")
    ///     .with_ios("NSPhotoLibraryUsageDescription")
    ///     .with_macos("NSPhotoLibraryUsageDescription")
    ///     .with_description("Access files")
    ///     .build();
    /// ```
    pub const fn with_android(mut self, android: &'static str) -> Self {
        self.android = Some(ConstStr::new(android));
        self
    }

    /// Set the iOS usage description key
    ///
    /// This key is used in the iOS Info.plist file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use permissions_core::{Permission, PermissionBuilder};
    ///
    /// const PERM: Permission = PermissionBuilder::custom()
    ///     .with_android("android.permission.READ_EXTERNAL_STORAGE")
    ///     .with_ios("NSPhotoLibraryUsageDescription")
    ///     .with_macos("NSPhotoLibraryUsageDescription")
    ///     .with_description("Access files")
    ///     .build();
    /// ```
    pub const fn with_ios(mut self, ios: &'static str) -> Self {
        self.ios = Some(ConstStr::new(ios));
        self
    }

    /// Set the macOS usage description key
    ///
    /// This key is used in the macOS Info.plist file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use permissions_core::{Permission, PermissionBuilder};
    ///
    /// const PERM: Permission = PermissionBuilder::custom()
    ///     .with_android("android.permission.READ_EXTERNAL_STORAGE")
    ///     .with_ios("NSPhotoLibraryUsageDescription")
    ///     .with_macos("NSPhotoLibraryUsageDescription")
    ///     .with_description("Access files")
    ///     .build();
    /// ```
    pub const fn with_macos(mut self, macos: &'static str) -> Self {
        self.macos = Some(ConstStr::new(macos));
        self
    }

    /// Set the user-facing description for this permission
    ///
    /// This description is used in platform manifests (Info.plist, AndroidManifest.xml)
    /// to explain why the permission is needed.
    pub const fn with_description(mut self, description: &'static str) -> Self {
        self.description = Some(ConstStr::new(description));
        self
    }

    /// Build the permission from the builder
    ///
    /// This validates that all required fields are set, then creates the `Permission` instance.
    ///
    /// # Panics
    ///
    /// This method will cause a compile-time error if any required field is missing:
    /// - `android` - Android permission string must be set
    /// - `ios` - iOS usage description key must be set
    /// - `macos` - macOS usage description key must be set
    /// - `description` - User-facing description must be set
    pub const fn build(self) -> Permission {
        let android = match self.android {
            Some(a) => a,
            None => panic!("CustomPermissionBuilder::build() requires android field to be set. Call .with_android() before .build()"),
        };
        let ios = match self.ios {
            Some(i) => i,
            None => panic!("CustomPermissionBuilder::build() requires ios field to be set. Call .with_ios() before .build()"),
        };
        let macos = match self.macos {
            Some(m) => m,
            None => panic!("CustomPermissionBuilder::build() requires macos field to be set. Call .with_macos() before .build()"),
        };
        let description = match self.description {
            Some(d) => d,
            None => panic!("CustomPermissionBuilder::build() requires description field to be set. Call .with_description() before .build()"),
        };

        let kind = PermissionKind::Custom {
            android,
            ios,
            macos,
        };
        let supported_platforms = kind.supported_platforms();

        Permission {
            kind,
            description,
            supported_platforms,
        }
    }
}

/// Builder for creating permissions with a const-friendly API
///
/// This builder is used for location and custom permissions that require
/// additional configuration. For simple permissions like Camera, Microphone,
/// and Notifications, use `Permission::new()` directly.
///
/// # Examples
///
/// ```rust
/// use permissions_core::{Permission, PermissionBuilder, LocationPrecision};
///
/// // Location permission with fine precision
/// const LOCATION: Permission = PermissionBuilder::location(LocationPrecision::Fine)
///     .with_description("Track your runs")
///     .build();
///
/// // Custom permission
/// const CUSTOM: Permission = PermissionBuilder::custom()
///     .with_android("android.permission.MY_PERMISSION")
///     .with_ios("NSMyUsageDescription")
///     .with_macos("NSMyUsageDescription")
///     .with_description("Custom permission")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct PermissionBuilder {
    /// The permission kind being built
    kind: Option<PermissionKind>,
    /// The user-facing description
    description: Option<ConstStr>,
}

impl PermissionBuilder {
    /// Create a new location permission builder with the specified precision
    ///
    /// # Examples
    ///
    /// ```rust
    /// use permissions_core::{Permission, PermissionBuilder, LocationPrecision};
    ///
    /// const LOCATION: Permission = PermissionBuilder::location(LocationPrecision::Fine)
    ///     .with_description("Track your runs")
    ///     .build();
    /// ```
    pub const fn location(precision: crate::LocationPrecision) -> Self {
        Self {
            kind: Some(PermissionKind::Location(precision)),
            description: None,
        }
    }

    /// Start building a custom permission with platform-specific identifiers
    ///
    /// Use the chained methods to specify each platform's identifier:
    /// - `.with_android()` - Android permission string
    /// - `.with_ios()` - iOS usage description key
    /// - `.with_macos()` - macOS usage description key
    ///
    /// # Examples
    ///
    /// ```rust
    /// use permissions_core::{Permission, PermissionBuilder};
    ///
    /// // Custom permission with all platforms
    /// const CUSTOM: Permission = PermissionBuilder::custom()
    ///     .with_android("android.permission.MY_PERMISSION")
    ///     .with_ios("NSMyUsageDescription")
    ///     .with_macos("NSMyUsageDescription")
    ///     .with_description("Custom permission")
    ///     .build();
    ///
    /// // Custom permission where iOS and macOS use the same key
    /// const PHOTO_LIBRARY: Permission = PermissionBuilder::custom()
    ///     .with_android("android.permission.READ_EXTERNAL_STORAGE")
    ///     .with_ios("NSPhotoLibraryUsageDescription")
    ///     .with_macos("NSPhotoLibraryUsageDescription")
    ///     .with_description("Access your photo library")
    ///     .build();
    /// ```
    pub const fn custom() -> CustomPermissionBuilder {
        CustomPermissionBuilder {
            android: None,
            ios: None,
            macos: None,
            description: None,
        }
    }

    /// Set the user-facing description for this permission
    ///
    /// This description is used in platform manifests (Info.plist, AndroidManifest.xml)
    /// to explain why the permission is needed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use permissions_core::{Permission, PermissionBuilder, LocationPrecision};
    ///
    /// const LOCATION: Permission = PermissionBuilder::location(LocationPrecision::Fine)
    ///     .with_description("Track your runs")
    ///     .build();
    /// ```
    pub const fn with_description(mut self, description: &'static str) -> Self {
        self.description = Some(ConstStr::new(description));
        self
    }

    /// Build the permission from the builder
    ///
    /// This validates that both the kind and description are set, then creates
    /// the `Permission` instance.
    ///
    /// # Panics
    ///
    /// This method will cause a compile-time error if any required field is missing:
    /// - `kind` - Permission kind must be set by calling `.location()` or `.custom()` before `.build()`
    /// - `description` - User-facing description must be set by calling `.with_description()` before `.build()`
    pub const fn build(self) -> Permission {
        let kind = match self.kind {
            Some(k) => k,
            None => panic!("PermissionBuilder::build() requires permission kind to be set. Call .location() or .custom() before .build()"),
        };

        let description = match self.description {
            Some(d) => d,
            None => panic!("PermissionBuilder::build() requires description field to be set. Call .with_description() before .build()"),
        };

        let supported_platforms = kind.supported_platforms();
        Permission {
            kind,
            description,
            supported_platforms,
        }
    }
}

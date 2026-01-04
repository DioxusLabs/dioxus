//! Unified manifest configuration for cross-platform app packaging.
//!
//! This module provides configuration structs for permissions and platform-specific
//! manifest customization. Permissions declared here are automatically mapped to
//! platform-specific identifiers (AndroidManifest.xml, Info.plist, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Unified Permissions
// ============================================================================

/// Unified permission configuration that maps to platform-specific identifiers.
///
/// Example:
/// ```toml
/// [permissions]
/// location = { precision = "fine", description = "Track your runs" }
/// camera = { description = "Take photos for your profile" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionsConfig {
    /// Location permission with precision level.
    /// Maps to ACCESS_FINE_LOCATION/ACCESS_COARSE_LOCATION on Android,
    /// NSLocationWhenInUseUsageDescription on iOS/macOS.
    #[serde(default)]
    pub location: Option<LocationPermission>,

    /// Camera access permission.
    #[serde(default)]
    pub camera: Option<SimplePermission>,

    /// Microphone access permission.
    #[serde(default)]
    pub microphone: Option<SimplePermission>,

    /// Push notifications permission.
    #[serde(default)]
    pub notifications: Option<SimplePermission>,

    /// Photo library access.
    #[serde(default)]
    pub photos: Option<StoragePermission>,

    /// Bluetooth connectivity.
    #[serde(default)]
    pub bluetooth: Option<SimplePermission>,

    /// Background location updates.
    #[serde(default, rename = "background-location")]
    pub background_location: Option<SimplePermission>,

    /// Contacts access.
    #[serde(default)]
    pub contacts: Option<StoragePermission>,

    /// Calendar access.
    #[serde(default)]
    pub calendar: Option<StoragePermission>,

    /// Biometric authentication (Face ID, fingerprint).
    #[serde(default)]
    pub biometrics: Option<SimplePermission>,

    /// NFC access.
    #[serde(default)]
    pub nfc: Option<SimplePermission>,

    /// Motion and fitness data.
    #[serde(default)]
    pub motion: Option<SimplePermission>,

    /// Health data access.
    #[serde(default)]
    pub health: Option<StoragePermission>,

    /// Speech recognition.
    #[serde(default)]
    pub speech: Option<SimplePermission>,

    /// Media library access.
    #[serde(default, rename = "media-library")]
    pub media_library: Option<SimplePermission>,

    /// Siri integration (iOS only).
    #[serde(default)]
    pub siri: Option<SimplePermission>,

    /// HomeKit integration (iOS only).
    #[serde(default)]
    pub homekit: Option<SimplePermission>,

    /// Local network access.
    #[serde(default, rename = "local-network")]
    pub local_network: Option<SimplePermission>,

    /// Nearby Wi-Fi devices (Android).
    #[serde(default, rename = "nearby-wifi")]
    pub nearby_wifi: Option<SimplePermission>,
}

/// Simple permission with just a description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimplePermission {
    /// User-facing description shown in permission dialogs.
    pub description: String,
}

/// Location permission with precision control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationPermission {
    /// Precision level: "fine" (GPS) or "coarse" (network-based).
    #[serde(default)]
    pub precision: LocationPrecision,

    /// User-facing description shown in permission dialogs.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum LocationPrecision {
    #[default]
    #[serde(rename = "fine")]
    Fine,
    #[serde(rename = "coarse")]
    Coarse,
}

/// Storage permission with access level control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePermission {
    /// Access level: "read", "write", or "read-write".
    #[serde(default)]
    pub access: StorageAccess,

    /// User-facing description shown in permission dialogs.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum StorageAccess {
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "write")]
    Write,
    #[default]
    #[serde(rename = "read-write")]
    ReadWrite,
}

/// Raw platform permission entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPermission {
    pub description: String,
}

// ============================================================================
// iOS Configuration
// ============================================================================

/// iOS-specific configuration.
///
/// Example:
/// ```toml
/// [ios]
/// deployment_target = "15.0"
///
/// [ios.entitlements]
/// app-groups = ["group.com.example.app"]
///
/// [ios.plist]
/// UIBackgroundModes = ["location", "fetch"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IosConfig {
    /// Minimum iOS deployment target (e.g., "15.0").
    #[serde(default)]
    pub deployment_target: Option<String>,

    /// Path to custom Info.plist to merge with generated.
    #[serde(default)]
    pub info_plist: Option<PathBuf>,

    /// iOS entitlements configuration.
    #[serde(default)]
    pub entitlements: IosEntitlements,

    /// Additional Info.plist keys to merge.
    #[serde(default)]
    pub plist: HashMap<String, serde_json::Value>,

    /// Raw XML injection points.
    #[serde(default)]
    pub raw: IosRawConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IosEntitlements {
    /// App groups for shared data.
    #[serde(default, rename = "app-groups")]
    pub app_groups: Vec<String>,

    /// Push notification environment: "development" or "production".
    #[serde(default, rename = "aps-environment")]
    pub aps_environment: Option<String>,

    /// Associated domains for universal links.
    #[serde(default, rename = "associated-domains")]
    pub associated_domains: Vec<String>,

    /// Enable iCloud container support.
    #[serde(default)]
    pub icloud: bool,

    /// Keychain access groups.
    #[serde(default, rename = "keychain-access-groups")]
    pub keychain_access_groups: Vec<String>,

    /// Enable Apple Pay.
    #[serde(default, rename = "apple-pay")]
    pub apple_pay: bool,

    /// Enable HealthKit.
    #[serde(default)]
    pub healthkit: bool,

    /// Enable HomeKit.
    #[serde(default)]
    pub homekit: bool,

    /// Additional entitlements.
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IosRawConfig {
    /// Raw XML to inject into Info.plist.
    #[serde(default)]
    pub info_plist: Option<String>,

    /// Raw XML to inject into entitlements.plist.
    #[serde(default)]
    pub entitlements: Option<String>,
}

// ============================================================================
// Android Configuration
// ============================================================================

/// Android-specific configuration.
///
/// Example:
/// ```toml
/// [android]
/// min_sdk = 24
/// target_sdk = 34
/// features = ["android.hardware.location.gps"]
///
/// [android.permissions]
/// "android.permission.FOREGROUND_SERVICE" = { description = "Background service" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AndroidConfig {
    /// Minimum SDK version.
    #[serde(default)]
    pub min_sdk: Option<u32>,

    /// Target SDK version.
    #[serde(default)]
    pub target_sdk: Option<u32>,

    /// Compile SDK version.
    #[serde(default)]
    pub compile_sdk: Option<u32>,

    /// Hardware/software features required.
    #[serde(default)]
    pub features: Vec<String>,

    /// Path to custom AndroidManifest.xml to merge.
    #[serde(default)]
    pub manifest: Option<PathBuf>,

    /// Gradle dependencies to add.
    #[serde(default)]
    pub gradle_dependencies: Vec<String>,

    /// Gradle plugins to apply.
    #[serde(default)]
    pub gradle_plugins: Vec<String>,

    /// ProGuard rule files.
    #[serde(default)]
    pub proguard_rules: Vec<PathBuf>,

    /// Additional Android permissions not in unified config.
    #[serde(default)]
    pub permissions: HashMap<String, RawPermission>,

    /// Raw XML injection points.
    #[serde(default)]
    pub raw: AndroidRawConfig,

    /// Application-level config.
    #[serde(default)]
    pub application: AndroidApplicationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AndroidRawConfig {
    /// Raw XML to inject into manifest (after permissions).
    #[serde(default)]
    pub manifest: Option<String>,

    /// Raw attributes for <application> element.
    #[serde(default)]
    pub application_attrs: Option<String>,

    /// Raw XML inside <application> element.
    #[serde(default)]
    pub application: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AndroidApplicationConfig {
    /// Enable cleartext (HTTP) traffic.
    #[serde(default)]
    pub uses_cleartext_traffic: Option<bool>,

    /// Application theme.
    #[serde(default)]
    pub theme: Option<String>,

    /// RTL layout support.
    #[serde(default)]
    pub supports_rtl: Option<bool>,

    /// Enable large heap.
    #[serde(default)]
    pub large_heap: Option<bool>,
}

// ============================================================================
// macOS Configuration
// ============================================================================

/// macOS-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacosConfig {
    /// Minimum macOS version (e.g., "11.0").
    #[serde(default)]
    pub minimum_system_version: Option<String>,

    /// Path to custom Info.plist.
    #[serde(default)]
    pub info_plist: Option<PathBuf>,

    /// Frameworks to embed.
    #[serde(default)]
    pub frameworks: Vec<String>,

    /// macOS entitlements.
    #[serde(default)]
    pub entitlements: MacosEntitlements,

    /// Additional Info.plist keys.
    #[serde(default)]
    pub plist: HashMap<String, serde_json::Value>,

    /// Raw injection points.
    #[serde(default)]
    pub raw: MacosRawConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacosEntitlements {
    /// Enable App Sandbox.
    #[serde(default, rename = "app-sandbox")]
    pub app_sandbox: Option<bool>,

    /// User-selected file access (read-write).
    #[serde(default, rename = "files-user-selected")]
    pub files_user_selected: Option<bool>,

    /// User-selected file access (read-only).
    #[serde(default, rename = "files-user-selected-readonly")]
    pub files_user_selected_readonly: Option<bool>,

    /// Outgoing network connections.
    #[serde(default, rename = "network-client")]
    pub network_client: Option<bool>,

    /// Incoming network connections.
    #[serde(default, rename = "network-server")]
    pub network_server: Option<bool>,

    /// Camera access.
    #[serde(default)]
    pub camera: Option<bool>,

    /// Microphone access.
    #[serde(default)]
    pub microphone: Option<bool>,

    /// USB access.
    #[serde(default)]
    pub usb: Option<bool>,

    /// Bluetooth access.
    #[serde(default)]
    pub bluetooth: Option<bool>,

    /// Printing.
    #[serde(default)]
    pub print: Option<bool>,

    /// Location services.
    #[serde(default)]
    pub location: Option<bool>,

    /// Address book access.
    #[serde(default)]
    pub addressbook: Option<bool>,

    /// Calendars access.
    #[serde(default)]
    pub calendars: Option<bool>,

    /// Disable library validation.
    #[serde(default, rename = "disable-library-validation")]
    pub disable_library_validation: Option<bool>,

    /// Allow JIT.
    #[serde(default, rename = "allow-jit")]
    pub allow_jit: Option<bool>,

    /// Allow unsigned executable memory.
    #[serde(default, rename = "allow-unsigned-executable-memory")]
    pub allow_unsigned_executable_memory: Option<bool>,

    /// Additional entitlements.
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacosRawConfig {
    /// Raw XML to inject into Info.plist.
    #[serde(default)]
    pub info_plist: Option<String>,

    /// Raw XML to inject into entitlements.plist.
    #[serde(default)]
    pub entitlements: Option<String>,
}

// ============================================================================
// Windows Configuration
// ============================================================================

/// Windows-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowsConfig {
    /// UWP/MSIX capabilities.
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Restricted capabilities.
    #[serde(default)]
    pub restricted_capabilities: Vec<String>,

    /// Device capabilities.
    #[serde(default)]
    pub device_capabilities: Vec<String>,
}

// ============================================================================
// Linux Configuration
// ============================================================================

/// Linux-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinuxConfig {
    /// Flatpak sandbox permissions.
    #[serde(default)]
    pub flatpak_permissions: Vec<String>,

    /// D-Bus interfaces to access.
    #[serde(default)]
    pub dbus_access: Vec<String>,

    /// Desktop entry categories.
    #[serde(default)]
    pub categories: Vec<String>,

    /// Desktop entry keywords.
    #[serde(default)]
    pub keywords: Vec<String>,

    /// MIME types the app can handle.
    #[serde(default)]
    pub mime_types: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_permissions() {
        let toml = r#"
            [permissions]
            location = { precision = "fine", description = "Track your runs" }
            camera = { description = "Take photos" }
        "#;

        #[derive(Deserialize)]
        struct Config {
            permissions: PermissionsConfig,
        }

        let config: Config = toml::from_str(toml).unwrap();
        let loc = config.permissions.location.unwrap();
        assert_eq!(loc.precision, LocationPrecision::Fine);
        assert_eq!(loc.description, "Track your runs");
        assert!(config.permissions.camera.is_some());
    }

    #[test]
    fn test_parse_ios_config() {
        let toml = r#"
            [ios]
            deployment_target = "15.0"

            [ios.entitlements]
            app-groups = ["group.com.example.app"]
        "#;

        #[derive(Deserialize)]
        struct Config {
            ios: IosConfig,
        }

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ios.deployment_target, Some("15.0".to_string()));
        assert_eq!(
            config.ios.entitlements.app_groups,
            vec!["group.com.example.app"]
        );
    }

    #[test]
    fn test_parse_android_config() {
        let toml = r#"
            [android]
            min_sdk = 24
            target_sdk = 34

            [android.permissions]
            "android.permission.FOREGROUND_SERVICE" = { description = "Background" }
        "#;

        #[derive(Deserialize)]
        struct Config {
            android: AndroidConfig,
        }

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.android.min_sdk, Some(24));
        assert!(config
            .android
            .permissions
            .contains_key("android.permission.FOREGROUND_SERVICE"));
    }
}

//! Unified manifest configuration for cross-platform app packaging.
//!
//! This module provides configuration structs for permissions and platform-specific
//! manifest customization. Permissions declared here are automatically mapped to
//! platform-specific identifiers (AndroidManifest.xml, Info.plist, etc.)
//!
//! ## JSON Schema Generation
//!
//! Generate a JSON schema for IDE autocomplete:
//! ```bash
//! dx config --schema > dioxus-schema.json
//! ```

use schemars::JsonSchema;
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimplePermission {
    /// User-facing description shown in permission dialogs.
    pub description: String,
}

// ============================================================================
// Unified Deep Linking
// ============================================================================

/// Unified deep linking configuration.
///
/// This provides a cross-platform interface for URL schemes and universal/app links.
/// Platform-specific overrides can be configured in `[ios]` and `[android]` sections.
///
/// Example:
/// ```toml
/// [deep_links]
/// schemes = ["myapp", "com.example.myapp"]
/// hosts = ["example.com", "*.example.com"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct DeepLinkConfig {
    /// Custom URL schemes (e.g., "myapp" for myapp://path).
    /// Maps to CFBundleURLSchemes on iOS/macOS and intent-filter on Android.
    #[serde(default)]
    pub schemes: Vec<String>,

    /// Universal link / App link hosts (e.g., "example.com").
    /// Maps to Associated Domains on iOS and App Links on Android.
    /// Supports wildcards like "*.example.com".
    #[serde(default)]
    pub hosts: Vec<String>,

    /// Path patterns for universal/app links (e.g., "/app/*", "/share/*").
    /// If empty, all paths are matched.
    #[serde(default)]
    pub paths: Vec<String>,
}

// ============================================================================
// Unified Background Modes
// ============================================================================

/// Unified background execution configuration.
///
/// This provides a cross-platform interface for background capabilities.
/// Platform-specific overrides can be configured in `[ios]` and `[android]` sections.
///
/// Example:
/// ```toml
/// [background]
/// location = true
/// audio = true
/// fetch = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct BackgroundConfig {
    /// Background location updates.
    /// iOS: UIBackgroundModes "location"
    /// Android: ACCESS_BACKGROUND_LOCATION permission
    #[serde(default)]
    pub location: bool,

    /// Background audio playback.
    /// iOS: UIBackgroundModes "audio"
    /// Android: FOREGROUND_SERVICE_MEDIA_PLAYBACK
    #[serde(default)]
    pub audio: bool,

    /// Background data fetch.
    /// iOS: UIBackgroundModes "fetch"
    /// Android: WorkManager or foreground service
    #[serde(default)]
    pub fetch: bool,

    /// Remote push notifications.
    /// iOS: UIBackgroundModes "remote-notification"
    /// Android: Firebase Cloud Messaging
    #[serde(default, rename = "remote-notifications")]
    pub remote_notifications: bool,

    /// VoIP calls.
    /// iOS: UIBackgroundModes "voip"
    /// Android: FOREGROUND_SERVICE_PHONE_CALL
    #[serde(default)]
    pub voip: bool,

    /// Bluetooth LE accessories.
    /// iOS: UIBackgroundModes "bluetooth-central" and "bluetooth-peripheral"
    /// Android: FOREGROUND_SERVICE_CONNECTED_DEVICE
    #[serde(default)]
    pub bluetooth: bool,

    /// External accessory communication.
    /// iOS: UIBackgroundModes "external-accessory"
    #[serde(default, rename = "external-accessory")]
    pub external_accessory: bool,

    /// Background processing tasks.
    /// iOS: UIBackgroundModes "processing"
    /// Android: WorkManager
    #[serde(default)]
    pub processing: bool,
}

/// Location permission with precision control.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocationPermission {
    /// Precision level: "fine" (GPS) or "coarse" (network-based).
    #[serde(default)]
    pub precision: LocationPrecision,

    /// User-facing description shown in permission dialogs.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, JsonSchema)]
pub enum LocationPrecision {
    #[default]
    #[serde(rename = "fine")]
    Fine,
    #[serde(rename = "coarse")]
    Coarse,
}

/// Storage permission with access level control.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoragePermission {
    /// Access level: "read", "write", or "read-write".
    #[serde(default)]
    pub access: StorageAccess,

    /// User-facing description shown in permission dialogs.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, JsonSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
/// identifier = "com.example.myapp.ios"  # Override bundle.identifier for iOS
///
/// [ios.entitlements]
/// app-groups = ["group.com.example.app"]
///
/// [ios.plist]
/// UIBackgroundModes = ["location", "fetch"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct IosConfig {
    // === Bundle settings (override [bundle] section) ===
    /// The app's identifier (e.g., "com.example.myapp").
    /// Overrides `bundle.identifier` for iOS builds.
    #[serde(default)]
    pub identifier: Option<String>,

    /// The app's publisher.
    /// Overrides `bundle.publisher` for iOS builds.
    #[serde(default)]
    pub publisher: Option<String>,

    /// Icons for the app.
    /// Overrides `bundle.icon` for iOS builds.
    #[serde(default)]
    pub icon: Option<Vec<String>>,

    /// Additional resources to bundle.
    /// Overrides `bundle.resources` for iOS builds.
    #[serde(default)]
    pub resources: Option<Vec<String>>,

    /// Copyright notice.
    /// Overrides `bundle.copyright` for iOS builds.
    #[serde(default)]
    pub copyright: Option<String>,

    /// App category.
    /// Overrides `bundle.category` for iOS builds.
    #[serde(default)]
    pub category: Option<String>,

    /// Short description.
    /// Overrides `bundle.short_description` for iOS builds.
    #[serde(default)]
    pub short_description: Option<String>,

    /// Long description.
    /// Overrides `bundle.long_description` for iOS builds.
    #[serde(default)]
    pub long_description: Option<String>,

    // === iOS-specific settings ===
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

    // === Platform-specific overrides (extend unified config) ===
    /// Additional URL schemes beyond unified `[deep_links]`.schemes.
    /// These are merged with the unified schemes.
    #[serde(default)]
    pub url_schemes: Vec<String>,

    /// Additional background modes beyond unified `[background]`.
    /// Valid values: "audio", "location", "voip", "fetch", "remote-notification",
    /// "newsstand-content", "external-accessory", "bluetooth-central",
    /// "bluetooth-peripheral", "processing"
    #[serde(default)]
    pub background_modes: Vec<String>,

    /// Document types the app can open.
    #[serde(default)]
    pub document_types: Vec<IosDocumentType>,

    /// Exported type identifiers (custom UTIs).
    #[serde(default)]
    pub exported_type_identifiers: Vec<IosTypeIdentifier>,

    /// Imported type identifiers.
    #[serde(default)]
    pub imported_type_identifiers: Vec<IosTypeIdentifier>,

    /// Widget extensions to compile and bundle.
    /// Each entry defines a Swift-based widget extension (.appex) that will be
    /// compiled and installed into the app's PlugIns folder.
    #[serde(default)]
    pub widget_extensions: Vec<WidgetExtensionConfig>,
}

/// Configuration for an iOS Widget Extension.
///
/// Widget extensions are compiled as Swift executables and bundled as .appex
/// bundles in the app's PlugIns folder.
///
/// Example in Dioxus.toml:
/// ```toml
/// [[ios.widget_extensions]]
/// source = "src/ios/widget"
/// display_name = "Location Widget"
/// bundle_id_suffix = "location-widget"
/// deployment_target = "16.2"
/// module_name = "GeolocationPlugin"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WidgetExtensionConfig {
    /// Path to the Swift package source directory (relative to project root).
    pub source: String,

    /// Display name for the widget (shown in system UI).
    pub display_name: String,

    /// Bundle ID suffix appended to the app's bundle identifier.
    /// For example, if the app is "com.example.app" and suffix is "location-widget",
    /// the widget bundle ID will be "com.example.app.location-widget".
    pub bundle_id_suffix: String,

    /// Minimum deployment target (e.g., "16.2").
    /// Defaults to the app's iOS deployment target if not specified.
    #[serde(default)]
    pub deployment_target: Option<String>,

    /// Swift module name for the widget.
    /// This MUST match the module name used by the main app's Swift plugin
    /// for ActivityKit type matching to work.
    pub module_name: String,
}

/// iOS document type declaration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IosDocumentType {
    /// Document type name.
    pub name: String,

    /// File extensions (e.g., ["txt", "md"]).
    #[serde(default)]
    pub extensions: Vec<String>,

    /// MIME types.
    #[serde(default)]
    pub mime_types: Vec<String>,

    /// UTI types.
    #[serde(default)]
    pub types: Vec<String>,

    /// Icon file name.
    #[serde(default)]
    pub icon: Option<String>,

    /// Role: "Editor", "Viewer", "Shell", or "None".
    #[serde(default)]
    pub role: Option<String>,
}

/// iOS Uniform Type Identifier declaration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IosTypeIdentifier {
    /// UTI identifier (e.g., "com.example.myformat").
    pub identifier: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Conforms to these UTIs.
    #[serde(default)]
    pub conforms_to: Vec<String>,

    /// File extensions.
    #[serde(default)]
    pub extensions: Vec<String>,

    /// MIME types.
    #[serde(default)]
    pub mime_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
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
/// identifier = "com.example.myapp.android"  # Override bundle.identifier for Android
/// features = ["android.hardware.location.gps"]
///
/// # Android signing configuration (previously in [bundle.android])
/// [android.signing]
/// jks_file = "keystore.jks"
/// jks_password = "password"
/// key_alias = "mykey"
/// key_password = "keypassword"
///
/// [android.permissions]
/// "android.permission.FOREGROUND_SERVICE" = { description = "Background service" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct AndroidConfig {
    // === Bundle settings (override [bundle] section) ===
    /// The app's identifier (e.g., "com.example.myapp").
    /// Overrides `bundle.identifier` for Android builds.
    #[serde(default)]
    pub identifier: Option<String>,

    /// The app's publisher.
    /// Overrides `bundle.publisher` for Android builds.
    #[serde(default)]
    pub publisher: Option<String>,

    /// Icons for the app.
    /// Overrides `bundle.icon` for Android builds.
    #[serde(default)]
    pub icon: Option<Vec<String>>,

    /// Additional resources to bundle.
    /// Overrides `bundle.resources` for Android builds.
    #[serde(default)]
    pub resources: Option<Vec<String>>,

    /// Copyright notice.
    /// Overrides `bundle.copyright` for Android builds.
    #[serde(default)]
    pub copyright: Option<String>,

    /// App category.
    /// Overrides `bundle.category` for Android builds.
    #[serde(default)]
    pub category: Option<String>,

    /// Short description.
    /// Overrides `bundle.short_description` for Android builds.
    #[serde(default)]
    pub short_description: Option<String>,

    /// Long description.
    /// Overrides `bundle.long_description` for Android builds.
    #[serde(default)]
    pub long_description: Option<String>,

    // === Android signing settings (previously in bundle.android) ===
    /// Android signing configuration for release builds.
    /// This replaces the deprecated `[bundle.android]` section.
    #[serde(default)]
    pub signing: Option<AndroidSigningConfig>,

    // === Android-specific settings ===
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

    // === Platform-specific overrides (extend unified config) ===
    /// Additional URL schemes beyond unified `[deep_links]`.schemes.
    /// These are merged with the unified schemes.
    #[serde(default)]
    pub url_schemes: Vec<String>,

    /// Intent filters for deep linking.
    /// These extend the unified `[deep_links]` configuration with Android-specific options.
    #[serde(default)]
    pub intent_filters: Vec<AndroidIntentFilter>,

    /// Foreground service types for background operations.
    /// Valid values: "camera", "connectedDevice", "dataSync", "health", "location",
    /// "mediaPlayback", "mediaProjection", "microphone", "phoneCall", "remoteMessaging",
    /// "shortService", "specialUse", "systemExempted"
    #[serde(default)]
    pub foreground_service_types: Vec<String>,

    /// Queries for package visibility (required for Android 11+).
    /// Specify packages or intents your app needs to query.
    #[serde(default)]
    pub queries: AndroidQueries,
}

/// Android signing configuration for release builds.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AndroidSigningConfig {
    /// Path to the Java keystore file.
    pub jks_file: PathBuf,

    /// Password for the keystore.
    pub jks_password: String,

    /// Alias of the key in the keystore.
    pub key_alias: String,

    /// Password for the key.
    pub key_password: String,
}

/// Android intent filter for deep linking.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AndroidIntentFilter {
    /// Actions (e.g., "android.intent.action.VIEW").
    #[serde(default)]
    pub actions: Vec<String>,

    /// Categories (e.g., "android.intent.category.DEFAULT", "android.intent.category.BROWSABLE").
    #[serde(default)]
    pub categories: Vec<String>,

    /// Data specifications.
    #[serde(default)]
    pub data: Vec<AndroidIntentData>,

    /// Auto-verify for App Links (requires HTTPS and assetlinks.json).
    #[serde(default)]
    pub auto_verify: bool,
}

/// Android intent data specification.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct AndroidIntentData {
    /// URL scheme (e.g., "https", "myapp").
    #[serde(default)]
    pub scheme: Option<String>,

    /// Host (e.g., "example.com").
    #[serde(default)]
    pub host: Option<String>,

    /// Port number.
    #[serde(default)]
    pub port: Option<String>,

    /// Path (exact match).
    #[serde(default)]
    pub path: Option<String>,

    /// Path prefix.
    #[serde(default)]
    pub path_prefix: Option<String>,

    /// Path pattern (with wildcards).
    #[serde(default)]
    pub path_pattern: Option<String>,

    /// MIME type.
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// Android package visibility queries.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct AndroidQueries {
    /// Package names to query.
    #[serde(default)]
    pub packages: Vec<String>,

    /// Intent actions to query.
    #[serde(default)]
    pub intents: Vec<AndroidQueryIntent>,
}

/// Android query intent specification.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AndroidQueryIntent {
    /// Action (e.g., "android.intent.action.SEND").
    pub action: String,

    /// Data scheme (e.g., "mailto").
    #[serde(default)]
    pub scheme: Option<String>,

    /// MIME type (e.g., "text/plain").
    #[serde(default)]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct AndroidRawConfig {
    /// Raw XML to inject into manifest (after permissions).
    #[serde(default)]
    pub manifest: Option<String>,

    /// Raw attributes for `<application>` element.
    #[serde(default)]
    pub application_attrs: Option<String>,

    /// Raw XML inside `<application>` element.
    #[serde(default)]
    pub application: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
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
///
/// Example:
/// ```toml
/// [macos]
/// minimum_system_version = "11.0"
/// identifier = "com.example.myapp.macos"  # Override bundle.identifier for macOS
///
/// # macOS signing (previously in [bundle.macos])
/// signing_identity = "Developer ID Application: My Company"
/// provider_short_name = "MYCOMPANY"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct MacosConfig {
    // === Bundle settings (override [bundle] section) ===
    /// The app's identifier (e.g., "com.example.myapp").
    /// Overrides `bundle.identifier` for macOS builds.
    #[serde(default)]
    pub identifier: Option<String>,

    /// The app's publisher.
    /// Overrides `bundle.publisher` for macOS builds.
    #[serde(default)]
    pub publisher: Option<String>,

    /// Icons for the app.
    /// Overrides `bundle.icon` for macOS builds.
    #[serde(default)]
    pub icon: Option<Vec<String>>,

    /// Additional resources to bundle.
    /// Overrides `bundle.resources` for macOS builds.
    #[serde(default)]
    pub resources: Option<Vec<String>>,

    /// Copyright notice.
    /// Overrides `bundle.copyright` for macOS builds.
    #[serde(default)]
    pub copyright: Option<String>,

    /// Short description.
    /// Overrides `bundle.short_description` for macOS builds.
    #[serde(default)]
    pub short_description: Option<String>,

    /// Long description.
    /// Overrides `bundle.long_description` for macOS builds.
    #[serde(default)]
    pub long_description: Option<String>,

    // === macOS bundle settings (previously in bundle.macos) ===
    /// The bundle version string (CFBundleVersion).
    #[serde(default)]
    pub bundle_version: Option<String>,

    /// The bundle short version string (CFBundleShortVersionString).
    #[serde(default)]
    pub bundle_name: Option<String>,

    /// The signing identity to use for code signing.
    /// E.g., "Developer ID Application: My Company (TEAMID)"
    #[serde(default)]
    pub signing_identity: Option<String>,

    /// The provider short name for notarization.
    #[serde(default)]
    pub provider_short_name: Option<String>,

    /// Path to custom entitlements file for code signing.
    /// This overrides the generated entitlements.
    #[serde(default)]
    pub entitlements_file: Option<String>,

    /// Exception domain for App Transport Security.
    #[serde(default)]
    pub exception_domain: Option<String>,

    /// License file to include in DMG.
    #[serde(default)]
    pub license: Option<String>,

    /// Preserve the hardened runtime version flag.
    /// Setting this to false is useful when using an ad-hoc signature.
    #[serde(default)]
    pub hardened_runtime: Option<bool>,

    /// Additional files to include in the app bundle.
    /// Maps the path in the Contents directory to the source file path.
    #[serde(default)]
    pub files: HashMap<PathBuf, PathBuf>,

    // === macOS-specific settings ===
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

    // === Platform-specific overrides (extend unified config) ===
    /// Additional URL schemes beyond unified `[deep_links]`.schemes.
    /// These are merged with the unified schemes.
    #[serde(default)]
    pub url_schemes: Vec<String>,

    /// Document types the app can open (uses same format as iOS).
    #[serde(default)]
    pub document_types: Vec<IosDocumentType>,

    /// Exported type identifiers (custom UTIs).
    #[serde(default)]
    pub exported_type_identifiers: Vec<IosTypeIdentifier>,

    /// Imported type identifiers.
    #[serde(default)]
    pub imported_type_identifiers: Vec<IosTypeIdentifier>,

    /// App category for the Mac App Store.
    /// E.g., "public.app-category.productivity"
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
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
///
/// Example:
/// ```toml
/// [windows]
/// identifier = "com.example.myapp.windows"  # Override bundle.identifier for Windows
///
/// # Windows installer settings (previously in [bundle.windows])
/// [windows.nsis]
/// install_mode = "PerMachine"
///
/// [windows.wix]
/// language = [["en-US", null]]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct WindowsConfig {
    // === Bundle settings (override [bundle] section) ===
    /// The app's identifier (e.g., "com.example.myapp").
    /// Overrides `bundle.identifier` for Windows builds.
    #[serde(default)]
    pub identifier: Option<String>,

    /// The app's publisher.
    /// Overrides `bundle.publisher` for Windows builds.
    #[serde(default)]
    pub publisher: Option<String>,

    /// Icons for the app.
    /// Overrides `bundle.icon` for Windows builds.
    #[serde(default)]
    pub icon: Option<Vec<String>>,

    /// Additional resources to bundle.
    /// Overrides `bundle.resources` for Windows builds.
    #[serde(default)]
    pub resources: Option<Vec<String>>,

    /// Copyright notice.
    /// Overrides `bundle.copyright` for Windows builds.
    #[serde(default)]
    pub copyright: Option<String>,

    /// App category.
    /// Overrides `bundle.category` for Windows builds.
    #[serde(default)]
    pub category: Option<String>,

    /// Short description.
    /// Overrides `bundle.short_description` for Windows builds.
    #[serde(default)]
    pub short_description: Option<String>,

    /// Long description.
    /// Overrides `bundle.long_description` for Windows builds.
    #[serde(default)]
    pub long_description: Option<String>,

    // === Windows bundle settings (previously in bundle.windows) ===
    /// Digest algorithm for code signing.
    #[serde(default)]
    pub digest_algorithm: Option<String>,

    /// Certificate thumbprint for code signing.
    #[serde(default)]
    pub certificate_thumbprint: Option<String>,

    /// Timestamp server URL for code signing.
    #[serde(default)]
    pub timestamp_url: Option<String>,

    /// Use TSP (RFC 3161) timestamp.
    #[serde(default)]
    pub tsp: Option<bool>,

    /// WiX installer settings.
    #[serde(default)]
    pub wix: Option<WindowsWixSettings>,

    /// NSIS installer settings.
    #[serde(default)]
    pub nsis: Option<WindowsNsisSettings>,

    /// Path to custom Windows icon.
    #[serde(default)]
    pub icon_path: Option<PathBuf>,

    /// WebView2 installation mode.
    #[serde(default)]
    pub webview_install_mode: Option<WindowsWebviewInstallMode>,

    /// Allow downgrades when installing.
    #[serde(default)]
    pub allow_downgrades: Option<bool>,

    /// Custom sign command.
    #[serde(default)]
    pub sign_command: Option<WindowsSignCommand>,

    // === Windows-specific settings ===
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

/// WiX installer settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct WindowsWixSettings {
    /// Languages and their locale paths.
    #[serde(default)]
    pub language: Vec<(String, Option<PathBuf>)>,

    /// Path to custom WiX template.
    #[serde(default)]
    pub template: Option<PathBuf>,

    /// WiX fragment files to include.
    #[serde(default)]
    pub fragment_paths: Vec<PathBuf>,

    /// Component group references.
    #[serde(default)]
    pub component_group_refs: Vec<String>,

    /// Component references.
    #[serde(default)]
    pub component_refs: Vec<String>,

    /// Feature group references.
    #[serde(default)]
    pub feature_group_refs: Vec<String>,

    /// Feature references.
    #[serde(default)]
    pub feature_refs: Vec<String>,

    /// Merge module references.
    #[serde(default)]
    pub merge_refs: Vec<String>,

    /// Skip WebView2 installation.
    #[serde(default)]
    pub skip_webview_install: Option<bool>,

    /// License file path.
    #[serde(default)]
    pub license: Option<PathBuf>,

    /// Enable elevated update task.
    #[serde(default)]
    pub enable_elevated_update_task: Option<bool>,

    /// Banner image path.
    #[serde(default)]
    pub banner_path: Option<PathBuf>,

    /// Dialog image path.
    #[serde(default)]
    pub dialog_image_path: Option<PathBuf>,

    /// FIPS compliant mode.
    #[serde(default)]
    pub fips_compliant: Option<bool>,

    /// MSI version string.
    #[serde(default)]
    pub version: Option<String>,

    /// MSI upgrade code (GUID).
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub upgrade_code: Option<uuid::Uuid>,
}

/// NSIS installer settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct WindowsNsisSettings {
    /// Path to custom NSIS template.
    #[serde(default)]
    pub template: Option<PathBuf>,

    /// License file path.
    #[serde(default)]
    pub license: Option<PathBuf>,

    /// Header image path.
    #[serde(default)]
    pub header_image: Option<PathBuf>,

    /// Sidebar image path.
    #[serde(default)]
    pub sidebar_image: Option<PathBuf>,

    /// Installer icon path.
    #[serde(default)]
    pub installer_icon: Option<PathBuf>,

    /// Installation mode: "CurrentUser", "PerMachine", or "Both".
    #[serde(default)]
    pub install_mode: Option<String>,

    /// Languages to include.
    #[serde(default)]
    pub languages: Option<Vec<String>>,

    /// Custom language files.
    #[serde(default)]
    pub custom_language_files: Option<HashMap<String, PathBuf>>,

    /// Display language selector.
    #[serde(default)]
    pub display_language_selector: Option<bool>,

    /// Start menu folder name.
    #[serde(default)]
    pub start_menu_folder: Option<String>,

    /// Installer hooks script path.
    #[serde(default)]
    pub installer_hooks: Option<PathBuf>,

    /// Minimum WebView2 version required.
    #[serde(default)]
    pub minimum_webview2_version: Option<String>,
}

/// WebView2 installation mode.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum WindowsWebviewInstallMode {
    /// Skip WebView2 installation.
    Skip,
    /// Download bootstrapper.
    DownloadBootstrapper {
        #[serde(default)]
        silent: bool,
    },
    /// Embed bootstrapper.
    EmbedBootstrapper {
        #[serde(default)]
        silent: bool,
    },
    /// Use offline installer.
    OfflineInstaller {
        #[serde(default)]
        silent: bool,
    },
    /// Use fixed runtime from path.
    FixedRuntime { path: PathBuf },
}

/// Custom sign command for Windows code signing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WindowsSignCommand {
    /// The command to run.
    pub cmd: String,
    /// Command arguments. Use "%1" as placeholder for binary path.
    pub args: Vec<String>,
}

// ============================================================================
// Linux Configuration
// ============================================================================

/// Linux-specific configuration.
///
/// Example:
/// ```toml
/// [linux]
/// identifier = "com.example.myapp.linux"  # Override bundle.identifier for Linux
/// categories = ["Utility"]
///
/// # Debian package settings (previously in [bundle.deb])
/// [linux.deb]
/// depends = ["libwebkit2gtk-4.0-37"]
/// section = "utils"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct LinuxConfig {
    // === Bundle settings (override [bundle] section) ===
    /// The app's identifier (e.g., "com.example.myapp").
    /// Overrides `bundle.identifier` for Linux builds.
    #[serde(default)]
    pub identifier: Option<String>,

    /// The app's publisher.
    /// Overrides `bundle.publisher` for Linux builds.
    #[serde(default)]
    pub publisher: Option<String>,

    /// Icons for the app.
    /// Overrides `bundle.icon` for Linux builds.
    #[serde(default)]
    pub icon: Option<Vec<String>>,

    /// Additional resources to bundle.
    /// Overrides `bundle.resources` for Linux builds.
    #[serde(default)]
    pub resources: Option<Vec<String>>,

    /// Copyright notice.
    /// Overrides `bundle.copyright` for Linux builds.
    #[serde(default)]
    pub copyright: Option<String>,

    /// App category.
    /// Overrides `bundle.category` for Linux builds.
    #[serde(default)]
    pub category: Option<String>,

    /// Short description.
    /// Overrides `bundle.short_description` for Linux builds.
    #[serde(default)]
    pub short_description: Option<String>,

    /// Long description.
    /// Overrides `bundle.long_description` for Linux builds.
    #[serde(default)]
    pub long_description: Option<String>,

    // === Debian package settings (previously in bundle.deb) ===
    /// Debian-specific package settings.
    #[serde(default)]
    pub deb: Option<LinuxDebSettings>,

    // === Linux-specific settings ===
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

/// Debian package settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct LinuxDebSettings {
    /// Package dependencies.
    #[serde(default)]
    pub depends: Option<Vec<String>>,

    /// Recommended packages.
    #[serde(default)]
    pub recommends: Option<Vec<String>>,

    /// Packages this provides.
    #[serde(default)]
    pub provides: Option<Vec<String>>,

    /// Package conflicts.
    #[serde(default)]
    pub conflicts: Option<Vec<String>>,

    /// Packages this replaces.
    #[serde(default)]
    pub replaces: Option<Vec<String>>,

    /// Additional files to include. Maps package path to source path.
    #[serde(default)]
    pub files: HashMap<PathBuf, PathBuf>,

    /// Path to custom desktop template.
    #[serde(default)]
    pub desktop_template: Option<PathBuf>,

    /// Debian section (e.g., "utils", "web").
    #[serde(default)]
    pub section: Option<String>,

    /// Package priority ("required", "important", "standard", "optional", "extra").
    #[serde(default)]
    pub priority: Option<String>,

    /// Path to changelog file.
    #[serde(default)]
    pub changelog: Option<PathBuf>,

    /// Pre-install script path.
    #[serde(default)]
    pub pre_install_script: Option<PathBuf>,

    /// Post-install script path.
    #[serde(default)]
    pub post_install_script: Option<PathBuf>,

    /// Pre-remove script path.
    #[serde(default)]
    pub pre_remove_script: Option<PathBuf>,

    /// Post-remove script path.
    #[serde(default)]
    pub post_remove_script: Option<PathBuf>,
}

// ============================================================================
// Schema Generation
// ============================================================================

/// Generate a JSON schema for the complete Dioxus.toml configuration.
///
/// This can be used for IDE autocomplete when editing Dioxus.toml files.
/// The schema includes all configuration: application, web, bundle, permissions,
/// platform-specific settings, and more.
///
/// Note: Default values are stripped and allOf wrappers simplified to prevent
/// stack overflow in some TOML LSP implementations (e.g., Taplo's WASM build).
pub fn generate_manifest_schema() -> schemars::schema::RootSchema {
    let mut schema = schemars::schema_for!(super::DioxusConfig);

    // Simplify schema to prevent Taplo WASM LSP stack overflow.
    // 1. Strip default values (large nested objects cause issues)
    // 2. Simplify allOf wrappers around single $refs
    simplify_schema(&mut schema.schema);
    for def in schema.definitions.values_mut() {
        if let schemars::schema::Schema::Object(obj) = def {
            simplify_schema(obj);
        }
    }

    schema
}

/// Recursively simplify a schema object for LSP compatibility.
/// - Removes default values (large nested objects cause stack overflow)
/// - Simplifies `allOf: [$ref]` to just `$ref` (reduces recursion depth)
fn simplify_schema(schema: &mut schemars::schema::SchemaObject) {
    // Remove the default value from this schema
    schema.metadata().default = None;

    // Simplify allOf with single $ref: { allOf: [{ $ref: "..." }] } -> { $ref: "..." }
    let mut ref_to_promote = None;
    if let Some(subschemas) = &schema.subschemas {
        if let Some(all_of) = &subschemas.all_of {
            if all_of.len() == 1 {
                if let schemars::schema::Schema::Object(inner) = &all_of[0] {
                    if inner.reference.is_some()
                        && inner.instance_type.is_none()
                        && inner.object.is_none()
                        && inner.array.is_none()
                        && inner.subschemas.is_none()
                    {
                        ref_to_promote = inner.reference.clone();
                    }
                }
            }
        }
    }
    if let Some(r) = ref_to_promote {
        schema.subschemas = None;
        schema.reference = Some(r);
    }

    // Process remaining subschemas
    if let Some(subschemas) = &mut schema.subschemas {
        if let Some(all_of) = &mut subschemas.all_of {
            for s in all_of {
                if let schemars::schema::Schema::Object(obj) = s {
                    simplify_schema(obj);
                }
            }
        }
        if let Some(any_of) = &mut subschemas.any_of {
            for s in any_of {
                if let schemars::schema::Schema::Object(obj) = s {
                    simplify_schema(obj);
                }
            }
        }
        if let Some(one_of) = &mut subschemas.one_of {
            for s in one_of {
                if let schemars::schema::Schema::Object(obj) = s {
                    simplify_schema(obj);
                }
            }
        }
    }

    // Process object properties
    if let Some(object) = &mut schema.object {
        for prop in object.properties.values_mut() {
            if let schemars::schema::Schema::Object(obj) = prop {
                simplify_schema(obj);
            }
        }
        if let Some(additional) = &mut object.additional_properties {
            if let schemars::schema::Schema::Object(obj) = additional.as_mut() {
                simplify_schema(obj);
            }
        }
    }

    // Process array items
    if let Some(array) = &mut schema.array {
        if let Some(items) = &mut array.items {
            match items {
                schemars::schema::SingleOrVec::Single(s) => {
                    if let schemars::schema::Schema::Object(obj) = s.as_mut() {
                        simplify_schema(obj);
                    }
                }
                schemars::schema::SingleOrVec::Vec(v) => {
                    for s in v {
                        if let schemars::schema::Schema::Object(obj) = s {
                            simplify_schema(obj);
                        }
                    }
                }
            }
        }
    }
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

    #[test]
    fn test_parse_deep_links() {
        let toml = r#"
            [deep_links]
            schemes = ["myapp", "com.example.app"]
            hosts = ["example.com", "*.example.com"]
            paths = ["/app/*", "/share/*"]
        "#;

        #[derive(Deserialize)]
        struct Config {
            deep_links: DeepLinkConfig,
        }

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.deep_links.schemes, vec!["myapp", "com.example.app"]);
        assert_eq!(
            config.deep_links.hosts,
            vec!["example.com", "*.example.com"]
        );
        assert_eq!(config.deep_links.paths, vec!["/app/*", "/share/*"]);
    }

    #[test]
    fn test_parse_background_modes() {
        let toml = r#"
            [background]
            location = true
            audio = true
            fetch = true
            remote-notifications = true
        "#;

        #[derive(Deserialize)]
        struct Config {
            background: BackgroundConfig,
        }

        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.background.location);
        assert!(config.background.audio);
        assert!(config.background.fetch);
        assert!(config.background.remote_notifications);
        assert!(!config.background.voip);
    }

    #[test]
    fn test_parse_ios_url_schemes_and_background() {
        let toml = r#"
            [ios]
            deployment_target = "15.0"
            url_schemes = ["myapp-ios"]
            background_modes = ["location", "fetch", "remote-notification"]

            [[ios.document_types]]
            name = "My Document"
            extensions = ["mydoc"]
            role = "Editor"
        "#;

        #[derive(Deserialize)]
        struct Config {
            ios: IosConfig,
        }

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.ios.url_schemes, vec!["myapp-ios"]);
        assert_eq!(
            config.ios.background_modes,
            vec!["location", "fetch", "remote-notification"]
        );
        assert_eq!(config.ios.document_types.len(), 1);
        assert_eq!(config.ios.document_types[0].name, "My Document");
    }

    #[test]
    fn test_parse_android_intent_filters() {
        let toml = r#"
            [android]
            min_sdk = 24
            url_schemes = ["myapp-android"]
            foreground_service_types = ["location", "mediaPlayback"]

            [[android.intent_filters]]
            actions = ["android.intent.action.VIEW"]
            categories = ["android.intent.category.DEFAULT", "android.intent.category.BROWSABLE"]
            auto_verify = true

            [[android.intent_filters.data]]
            scheme = "https"
            host = "example.com"
            path_prefix = "/app"
        "#;

        #[derive(Deserialize)]
        struct Config {
            android: AndroidConfig,
        }

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.android.url_schemes, vec!["myapp-android"]);
        assert_eq!(
            config.android.foreground_service_types,
            vec!["location", "mediaPlayback"]
        );
        assert_eq!(config.android.intent_filters.len(), 1);
        assert!(config.android.intent_filters[0].auto_verify);
    }

    #[test]
    fn test_parse_macos_url_schemes() {
        let toml = r#"
            [macos]
            minimum_system_version = "11.0"
            url_schemes = ["myapp-macos"]
            category = "public.app-category.productivity"

            [[macos.document_types]]
            name = "My Format"
            extensions = ["myfmt"]
        "#;

        #[derive(Deserialize)]
        struct Config {
            macos: MacosConfig,
        }

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.macos.url_schemes, vec!["myapp-macos"]);
        assert_eq!(
            config.macos.category,
            Some("public.app-category.productivity".to_string())
        );
        assert_eq!(config.macos.document_types.len(), 1);
    }

    #[test]
    fn test_generate_schema() {
        let schema = generate_manifest_schema();
        let json = serde_json::to_string_pretty(&schema).unwrap();

        // Verify the schema contains all top-level DioxusConfig types
        assert!(json.contains("ApplicationConfig"));
        assert!(json.contains("WebConfig"));
        assert!(json.contains("BundleConfig"));
        assert!(json.contains("ComponentConfig"));
        assert!(json.contains("PermissionsConfig"));
        assert!(json.contains("DeepLinkConfig"));
        assert!(json.contains("BackgroundConfig"));
        assert!(json.contains("IosConfig"));
        assert!(json.contains("AndroidConfig"));
        assert!(json.contains("MacosConfig"));
        assert!(json.contains("WindowsConfig"));
        assert!(json.contains("LinuxConfig"));

        // Verify some specific properties exist
        assert!(json.contains("location"));
        assert!(json.contains("camera"));
        assert!(json.contains("deployment_target"));
        assert!(json.contains("min_sdk"));

        // Verify application config properties
        assert!(json.contains("asset_dir"));
        assert!(json.contains("public_dir"));

        // Verify web config properties
        assert!(json.contains("pre_compress"));
        assert!(json.contains("wasm_opt"));

        // Verify bundle config properties
        assert!(json.contains("identifier"));
        assert!(json.contains("publisher"));
    }
}

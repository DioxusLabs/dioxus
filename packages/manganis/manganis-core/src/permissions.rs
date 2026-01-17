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

    /// A permission declaration for the application
    Permission(Permission),

    /// Android plugin metadata (prebuilt artifacts + Gradle deps)
    AndroidArtifact(AndroidArtifactMetadata),

    /// Swift package metadata (SPM location + product)
    SwiftPackage(SwiftPackageMetadata),

    /// Apple Widget Extension (.appex) to bundle with the app
    AppleWidgetExtension(AppleWidgetExtensionMetadata),
}

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

    /// Deserialize a permission from the bytes emitted into linker sections.
    /// This helper mirrors what the CLI performs when it scans the binary and
    /// allows runtime consumers to interpret the serialized metadata as well.
    pub fn from_embedded(bytes: &[u8]) -> Option<Self> {
        const SYMBOL_SIZE: usize = std::mem::size_of::<SymbolData>();
        let (_, symbol) =
            unsafe { const_serialize::deserialize_const_raw::<SYMBOL_SIZE, SymbolData>(bytes) }?;
        match symbol {
            SymbolData::Permission(permission) => Some(permission),
            _ => None,
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
    /// Set the Android permission string.
    ///
    /// Call this when the permission applies to Android. Omit it for iOS/macOS-only permissions.
    ///
    /// # Examples
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
    /// Call this when the permission applies to iOS. Omit it when not needed.
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
    /// Call this when the permission applies to macOS. Omit it when not needed.
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
    /// - `description` - User-facing description must be set
    /// - `android`/`ios`/`macos` - At least one platform identifier must be provided
    pub const fn build(self) -> Permission {
        let description = match self.description {
            Some(d) => d,
            None => panic!("CustomPermissionBuilder::build() requires description field to be set. Call .with_description() before .build()"),
        };

        if self.android.is_none() && self.ios.is_none() && self.macos.is_none() {
            panic!("CustomPermissionBuilder::build() requires at least one platform identifier. Call .with_android(), .with_ios(), or .with_macos() before .build()");
        }

        let android = match self.android {
            Some(value) => value,
            None => ConstStr::new(""),
        };
        let ios = match self.ios {
            Some(value) => value,
            None => ConstStr::new(""),
        };
        let macos = match self.macos {
            Some(value) => value,
            None => ConstStr::new(""),
        };

        let kind = PermissionKind::Custom {
            android,
            ios,
            macos,
            android_enabled: self.android.is_some(),
            ios_enabled: self.ios.is_some(),
            macos_enabled: self.macos.is_some(),
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
    pub const fn coarse_location() -> Self {
        Self {
            kind: Some(PermissionKind::CoarseLocation),
            description: None,
        }
    }

    /// Create a new location permission builder with the specified precision
    ////
    /// # Examples
    ////
    /// ```rust
    /// use permissions_core::{Permission, PermissionBuilder, LocationPrecision};
    ////
    /// const LOCATION: Permission = PermissionBuilder::fine_location()
    ///     .with_description("Track your runs")
    ///     .build();
    /// ```
    pub const fn fine_location() -> Self {
        Self {
            kind: Some(PermissionKind::FineLocation),
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

#[cfg(test)]
mod tests {
    use super::*;
    use const_serialize::{serialize_const, ConstVec};

    #[test]
    fn custom_permission_with_partial_platforms() {
        let permission = PermissionBuilder::custom()
            .with_android("android.permission.CAMERA")
            .with_description("Camera access on Android")
            .build();

        assert!(permission.supports_platform(Platform::Android));
        assert!(!permission.supports_platform(Platform::Ios));
        assert!(!permission.supports_platform(Platform::Macos));
    }

    #[test]
    #[should_panic(
        expected = "CustomPermissionBuilder::build() requires at least one platform identifier"
    )]
    fn custom_permission_requires_platform() {
        let _ = PermissionBuilder::custom()
            .with_description("Missing identifiers")
            .build();
    }

    #[test]
    fn deserialize_permission_from_embedded_bytes() {
        let permission = Permission::new(PermissionKind::Camera, "Camera access");
        let buffer = serialize_const(&SymbolData::Permission(permission), ConstVec::<u8>::new());
        let decoded = Permission::from_embedded(buffer.as_ref()).expect("permission decoded");
        assert_eq!(decoded.description(), permission.description());
        assert!(decoded.supports_platform(Platform::Android));
    }
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

/// Core permission kinds that map to platform-specific requirements
///
/// Only tested and verified permissions are included. For untested permissions,
/// use the `Custom` variant with platform-specific identifiers.
#[repr(C, u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeConst)]
#[allow(clippy::large_enum_variant)] // Custom variant contains large ConstStr fields needed for const serialization
pub enum PermissionKind {
    // ===== Camera & Microphone =====
    /// Camera access for taking photos/videos
    Camera,

    /// Microphone access for audio recording
    Microphone,

    // ===== Location =====
    /// Coarse location (city-level accuracy)
    CoarseLocation,

    /// Fine/precise location (GPS-level accuracy)
    FineLocation,

    /// Background location updates when app is not in foreground
    BackgroundLocation,

    // ===== Notifications =====
    /// Push notifications
    Notifications,

    // ===== Storage =====
    /// Read from external/shared storage (Android legacy)
    ReadStorage,

    /// Write to external/shared storage (Android legacy)
    WriteStorage,

    /// Read photos/images from media library
    ReadPhotos,

    /// Add/write photos to media library
    WritePhotos,

    /// Read videos from media library
    ReadVideos,

    /// Read audio files from media library
    ReadAudio,

    /// Full disk access (macOS only)
    FullDiskAccess,

    /// Access to Desktop folder (macOS only)
    DesktopFolder,

    /// Access to Documents folder (macOS only)
    DocumentsFolder,

    /// Access to Downloads folder (macOS only)
    DownloadsFolder,

    // ===== Contacts & Calendar =====
    /// Read contacts from address book
    ReadContacts,

    /// Write/modify contacts in address book
    WriteContacts,

    /// Read calendar events
    ReadCalendar,

    /// Write/modify calendar events
    WriteCalendar,

    /// Access to reminders (iOS/macOS)
    Reminders,

    // ===== Phone & SMS (primarily Android) =====
    /// Read phone state (IMEI, phone number, etc.)
    PhoneState,

    /// Make phone calls
    MakeCalls,

    /// Read call history/log
    ReadCallLog,

    /// Write to call history/log
    WriteCallLog,

    /// Send SMS messages
    SendSms,

    /// Read SMS messages
    ReadSms,

    /// Receive SMS messages (background)
    ReceiveSms,

    // ===== Connectivity =====
    /// Bluetooth connection/pairing
    Bluetooth,

    /// Bluetooth device scanning
    BluetoothScan,

    /// Bluetooth advertising (peripheral mode)
    BluetoothAdvertise,

    /// NFC (Near Field Communication)
    Nfc,

    /// Local network access (iOS)
    LocalNetwork,

    /// Discover nearby WiFi devices (Android)
    NearbyWifiDevices,

    /// Internet access (Android)
    Internet,

    // ===== Health & Sensors =====
    /// Body sensors (heart rate, etc.)
    BodySensors,

    /// Motion and fitness activity data
    MotionAndFitness,

    /// Activity recognition (walking, driving, etc.)
    ActivityRecognition,

    /// Read health data
    ReadHealth,

    /// Write health data
    WriteHealth,

    // ===== Desktop (macOS) =====
    /// Screen capture/recording (macOS)
    ScreenCapture,

    /// Accessibility features (macOS)
    Accessibility,

    /// Input monitoring - keyboard/mouse events (macOS)
    InputMonitoring,

    /// Automation/AppleScript control of other apps (macOS)
    Automation,

    // ===== Privacy =====
    /// App tracking / advertising ID
    AppTracking,

    /// Biometric authentication (Face ID, Touch ID, fingerprint)
    Biometrics,

    // ===== Media =====
    /// Media library access (Apple Music, etc.)
    MediaLibrary,

    /// Speech recognition
    SpeechRecognition,

    /// Siri integration
    Siri,

    /// HomeKit smart home access
    HomeKit,

    /// Vibration/haptic feedback (Android)
    Vibration,

    /// Custom permission with platform-specific identifiers
    Custom {
        android: ConstStr,
        ios: ConstStr,
        macos: ConstStr,
        android_enabled: bool,
        ios_enabled: bool,
        macos_enabled: bool,
    },
}

impl PermissionKind {
    /// Get the platform-specific permission identifiers for this permission kind
    pub const fn platform_identifiers(&self) -> PlatformIdentifiers {
        match self {
            // ===== Camera & Microphone =====
            PermissionKind::Camera => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.CAMERA")),
                ios: Some(ConstStr::new("NSCameraUsageDescription")),
                macos: Some(ConstStr::new("NSCameraUsageDescription")),
            },
            PermissionKind::Microphone => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.RECORD_AUDIO")),
                ios: Some(ConstStr::new("NSMicrophoneUsageDescription")),
                macos: Some(ConstStr::new("NSMicrophoneUsageDescription")),
            },

            // ===== Location =====
            PermissionKind::CoarseLocation => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.ACCESS_COARSE_LOCATION")),
                ios: Some(ConstStr::new("NSLocationWhenInUseUsageDescription")),
                macos: Some(ConstStr::new("NSLocationUsageDescription")),
            },
            PermissionKind::FineLocation => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.ACCESS_FINE_LOCATION")),
                ios: Some(ConstStr::new("NSLocationWhenInUseUsageDescription")),
                macos: Some(ConstStr::new("NSLocationUsageDescription")),
            },
            PermissionKind::BackgroundLocation => PlatformIdentifiers {
                android: Some(ConstStr::new(
                    "android.permission.ACCESS_BACKGROUND_LOCATION",
                )),
                ios: Some(ConstStr::new(
                    "NSLocationAlwaysAndWhenInUseUsageDescription",
                )),
                macos: Some(ConstStr::new("NSLocationAlwaysUsageDescription")),
            },

            // ===== Notifications =====
            PermissionKind::Notifications => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.POST_NOTIFICATIONS")),
                ios: None,   // Runtime request only
                macos: None, // Runtime request only
            },

            // ===== Storage =====
            PermissionKind::ReadStorage => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_EXTERNAL_STORAGE")),
                ios: None,
                macos: None,
            },
            PermissionKind::WriteStorage => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.WRITE_EXTERNAL_STORAGE")),
                ios: None,
                macos: None,
            },
            PermissionKind::ReadPhotos => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_MEDIA_IMAGES")),
                ios: Some(ConstStr::new("NSPhotoLibraryUsageDescription")),
                macos: Some(ConstStr::new("NSPhotoLibraryUsageDescription")),
            },
            PermissionKind::WritePhotos => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_MEDIA_IMAGES")),
                ios: Some(ConstStr::new("NSPhotoLibraryAddUsageDescription")),
                macos: Some(ConstStr::new("NSPhotoLibraryAddUsageDescription")),
            },
            PermissionKind::ReadVideos => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_MEDIA_VIDEO")),
                ios: Some(ConstStr::new("NSPhotoLibraryUsageDescription")),
                macos: Some(ConstStr::new("NSPhotoLibraryUsageDescription")),
            },
            PermissionKind::ReadAudio => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_MEDIA_AUDIO")),
                ios: Some(ConstStr::new("NSAppleMusicUsageDescription")),
                macos: Some(ConstStr::new("NSAppleMusicUsageDescription")),
            },
            PermissionKind::FullDiskAccess => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: Some(ConstStr::new("NSSystemAdministrationUsageDescription")),
            },
            PermissionKind::DesktopFolder => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: Some(ConstStr::new("NSDesktopFolderUsageDescription")),
            },
            PermissionKind::DocumentsFolder => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: Some(ConstStr::new("NSDocumentsFolderUsageDescription")),
            },
            PermissionKind::DownloadsFolder => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: Some(ConstStr::new("NSDownloadsFolderUsageDescription")),
            },

            // ===== Contacts & Calendar =====
            PermissionKind::ReadContacts => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_CONTACTS")),
                ios: Some(ConstStr::new("NSContactsUsageDescription")),
                macos: Some(ConstStr::new("NSContactsUsageDescription")),
            },
            PermissionKind::WriteContacts => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.WRITE_CONTACTS")),
                ios: Some(ConstStr::new("NSContactsUsageDescription")),
                macos: Some(ConstStr::new("NSContactsUsageDescription")),
            },
            PermissionKind::ReadCalendar => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_CALENDAR")),
                ios: Some(ConstStr::new("NSCalendarsFullAccessUsageDescription")),
                macos: Some(ConstStr::new("NSCalendarsFullAccessUsageDescription")),
            },
            PermissionKind::WriteCalendar => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.WRITE_CALENDAR")),
                ios: Some(ConstStr::new("NSCalendarsFullAccessUsageDescription")),
                macos: Some(ConstStr::new("NSCalendarsFullAccessUsageDescription")),
            },
            PermissionKind::Reminders => PlatformIdentifiers {
                android: None,
                ios: Some(ConstStr::new("NSRemindersFullAccessUsageDescription")),
                macos: Some(ConstStr::new("NSRemindersFullAccessUsageDescription")),
            },

            // ===== Phone & SMS (primarily Android) =====
            PermissionKind::PhoneState => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_PHONE_STATE")),
                ios: None,
                macos: None,
            },
            PermissionKind::MakeCalls => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.CALL_PHONE")),
                ios: None,
                macos: None,
            },
            PermissionKind::ReadCallLog => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_CALL_LOG")),
                ios: None,
                macos: None,
            },
            PermissionKind::WriteCallLog => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.WRITE_CALL_LOG")),
                ios: None,
                macos: None,
            },
            PermissionKind::SendSms => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.SEND_SMS")),
                ios: None,
                macos: None,
            },
            PermissionKind::ReadSms => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_SMS")),
                ios: None,
                macos: None,
            },
            PermissionKind::ReceiveSms => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.RECEIVE_SMS")),
                ios: None,
                macos: None,
            },

            // ===== Connectivity =====
            PermissionKind::Bluetooth => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.BLUETOOTH_CONNECT")),
                ios: Some(ConstStr::new("NSBluetoothAlwaysUsageDescription")),
                macos: Some(ConstStr::new("NSBluetoothAlwaysUsageDescription")),
            },
            PermissionKind::BluetoothScan => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.BLUETOOTH_SCAN")),
                ios: Some(ConstStr::new("NSBluetoothPeripheralUsageDescription")),
                macos: Some(ConstStr::new("NSBluetoothPeripheralUsageDescription")),
            },
            PermissionKind::BluetoothAdvertise => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.BLUETOOTH_ADVERTISE")),
                ios: Some(ConstStr::new("NSBluetoothPeripheralUsageDescription")),
                macos: Some(ConstStr::new("NSBluetoothPeripheralUsageDescription")),
            },
            PermissionKind::Nfc => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.NFC")),
                ios: Some(ConstStr::new("NFCReaderUsageDescription")),
                macos: None,
            },
            PermissionKind::LocalNetwork => PlatformIdentifiers {
                android: None,
                ios: Some(ConstStr::new("NSLocalNetworkUsageDescription")),
                macos: None,
            },
            PermissionKind::NearbyWifiDevices => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.NEARBY_WIFI_DEVICES")),
                ios: None,
                macos: None,
            },
            PermissionKind::Internet => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.INTERNET")),
                ios: None,
                macos: None,
            },

            // ===== Health & Sensors =====
            PermissionKind::BodySensors => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.BODY_SENSORS")),
                ios: None,
                macos: None,
            },
            PermissionKind::MotionAndFitness => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.ACTIVITY_RECOGNITION")),
                ios: Some(ConstStr::new("NSMotionUsageDescription")),
                macos: None,
            },
            PermissionKind::ActivityRecognition => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.ACTIVITY_RECOGNITION")),
                ios: Some(ConstStr::new("NSMotionUsageDescription")),
                macos: None,
            },
            PermissionKind::ReadHealth => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.BODY_SENSORS")),
                ios: Some(ConstStr::new("NSHealthShareUsageDescription")),
                macos: None,
            },
            PermissionKind::WriteHealth => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.BODY_SENSORS")),
                ios: Some(ConstStr::new("NSHealthUpdateUsageDescription")),
                macos: None,
            },

            // ===== Desktop (macOS) =====
            PermissionKind::ScreenCapture => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: Some(ConstStr::new("NSScreenCaptureUsageDescription")),
            },
            PermissionKind::Accessibility => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: Some(ConstStr::new("NSAccessibilityUsageDescription")),
            },
            PermissionKind::InputMonitoring => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: Some(ConstStr::new("NSInputMonitoringUsageDescription")),
            },
            PermissionKind::Automation => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: Some(ConstStr::new("NSAppleEventsUsageDescription")),
            },

            // ===== Privacy =====
            PermissionKind::AppTracking => PlatformIdentifiers {
                android: Some(ConstStr::new("com.google.android.gms.permission.AD_ID")),
                ios: Some(ConstStr::new("NSUserTrackingUsageDescription")),
                macos: None,
            },
            PermissionKind::Biometrics => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.USE_BIOMETRIC")),
                ios: Some(ConstStr::new("NSFaceIDUsageDescription")),
                macos: Some(ConstStr::new("NSFaceIDUsageDescription")),
            },

            // ===== Media =====
            PermissionKind::MediaLibrary => PlatformIdentifiers {
                android: None,
                ios: Some(ConstStr::new("NSAppleMusicUsageDescription")),
                macos: Some(ConstStr::new("NSAppleMusicUsageDescription")),
            },
            PermissionKind::SpeechRecognition => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.RECORD_AUDIO")),
                ios: Some(ConstStr::new("NSSpeechRecognitionUsageDescription")),
                macos: Some(ConstStr::new("NSSpeechRecognitionUsageDescription")),
            },
            PermissionKind::Siri => PlatformIdentifiers {
                android: None,
                ios: Some(ConstStr::new("NSSiriUsageDescription")),
                macos: Some(ConstStr::new("NSSiriUsageDescription")),
            },
            PermissionKind::HomeKit => PlatformIdentifiers {
                android: None,
                ios: Some(ConstStr::new("NSHomeKitUsageDescription")),
                macos: Some(ConstStr::new("NSHomeKitUsageDescription")),
            },
            PermissionKind::Vibration => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.VIBRATE")),
                ios: None,
                macos: None,
            },

            // ===== Custom =====
            PermissionKind::Custom {
                android,
                ios,
                macos,
                android_enabled,
                ios_enabled,
                macos_enabled,
            } => PlatformIdentifiers {
                android: if *android_enabled {
                    Some(*android)
                } else {
                    None
                },
                ios: if *ios_enabled { Some(*ios) } else { None },
                macos: if *macos_enabled { Some(*macos) } else { None },
            },
        }
    }

    /// Get the platforms that support this permission kind
    pub const fn supported_platforms(&self) -> PlatformFlags {
        let identifiers = self.platform_identifiers();
        let mut flags = PlatformFlags::new();

        if identifiers.android.is_some() {
            flags = flags.with_platform(Platform::Android);
        }
        if identifiers.ios.is_some() {
            flags = flags.with_platform(Platform::Ios);
        }
        if identifiers.macos.is_some() {
            flags = flags.with_platform(Platform::Macos);
        }

        flags
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

/// Metadata for an Apple Widget Extension (.appex) that should be bundled with the app.
///
/// Widget extensions provide lock screen and Dynamic Island UI for Live Activities
/// on iOS 16.2+ and watchOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeConst)]
pub struct AppleWidgetExtensionMetadata {
    /// Path to the Swift package containing the widget extension
    pub package_path: ConstStr,
    /// Display name shown in the widget gallery
    pub display_name: ConstStr,
    /// Suffix for the bundle identifier (e.g., "location-widget" -> com.app.id.location-widget)
    pub bundle_id_suffix: ConstStr,
    /// Minimum iOS deployment target (e.g., "17.0")
    pub deployment_target: ConstStr,
    /// Swift module name for ActivityKit type matching.
    /// This MUST match the module name used by the main app's Swift plugin
    /// for Live Activity types to be recognized (e.g., "GeolocationPlugin").
    pub module_name: ConstStr,
}

impl AppleWidgetExtensionMetadata {
    pub const fn new(
        package_path: &'static str,
        display_name: &'static str,
        bundle_id_suffix: &'static str,
        deployment_target: &'static str,
        module_name: &'static str,
    ) -> Self {
        Self {
            package_path: ConstStr::new(package_path),
            display_name: ConstStr::new(display_name),
            bundle_id_suffix: ConstStr::new(bundle_id_suffix),
            deployment_target: ConstStr::new(deployment_target),
            module_name: ConstStr::new(module_name),
        }
    }
}

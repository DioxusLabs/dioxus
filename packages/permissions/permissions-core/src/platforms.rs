use const_serialize::{ConstStr, SerializeConst};

/// Platform categories for permission mapping
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeConst)]
pub enum Platform {
    /// Mobile platforms
    Android,
    Ios,
    /// Desktop platforms
    Macos,
    Windows,
    Linux,
    /// Web platform
    Web,
}

/// Bit flags for supported platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeConst)]
pub struct PlatformFlags(u8);

impl PlatformFlags {
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn with_platform(mut self, platform: Platform) -> Self {
        self.0 |= 1 << platform as u8;
        self
    }

    pub const fn supports(&self, platform: Platform) -> bool {
        (self.0 & (1 << platform as u8)) != 0
    }

    pub const fn all() -> Self {
        Self(0b111111) // All 6 platforms
    }

    pub const fn mobile() -> Self {
        Self(0b000011) // Android + iOS
    }

    pub const fn desktop() -> Self {
        Self(0b011100) // macOS + Windows + Linux
    }

    pub const fn cross_platform() -> Self {
        Self(0b000111) // Android + iOS + Web
    }
}

/// Location precision for location-based permissions
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeConst)]
pub enum LocationPrecision {
    /// Fine location (GPS-level accuracy)
    Fine,
    /// Coarse location (network-based accuracy)
    Coarse,
}

/// Core permission kinds that map to platform-specific requirements
#[repr(C, u8)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SerializeConst)]
pub enum PermissionKind {
    // Cross-platform permissions
    Camera,
    Location(LocationPrecision),
    Microphone,
    PhotoLibrary,
    Contacts,
    Calendar,
    Bluetooth,
    Notifications,
    FileSystem,
    Network,

    // Mobile-specific permissions
    /// Android: READ_SMS, iOS: No equivalent
    Sms,
    /// Android: READ_PHONE_STATE, iOS: No equivalent  
    PhoneState,
    /// Android: CALL_PHONE, iOS: No equivalent
    PhoneCall,
    /// Android: SYSTEM_ALERT_WINDOW, iOS: No equivalent
    SystemAlertWindow,

    // iOS/macOS-specific permissions
    /// iOS: NSUserTrackingUsageDescription
    UserTracking,
    /// iOS: NSFaceIDUsageDescription
    FaceId,
    /// iOS: NSLocalNetworkUsageDescription
    LocalNetwork,

    // Windows-specific permissions
    /// Windows: appointments capability
    Appointments,
    /// Windows: phoneCall capability
    WindowsPhoneCall,
    /// Windows: enterpriseAuthentication capability
    EnterpriseAuth,

    // Web-specific permissions
    /// Web: clipboard-read, clipboard-write
    Clipboard,
    /// Web: payment-handler
    Payment,
    /// Web: screen-wake-lock
    ScreenWakeLock,

    // Custom permissions for extensibility
    Custom {
        android: ConstStr,
        ios: ConstStr,
        macos: ConstStr,
        windows: ConstStr,
        linux: ConstStr,
        web: ConstStr,
    },
}

impl PermissionKind {
    /// Get the platform-specific permission identifiers for this permission kind
    pub const fn platform_identifiers(&self) -> PlatformIdentifiers {
        match self {
            PermissionKind::Camera => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.CAMERA")),
                ios: Some(ConstStr::new("NSCameraUsageDescription")),
                macos: Some(ConstStr::new("NSCameraUsageDescription")),
                windows: Some(ConstStr::new("webcam")),
                linux: None,
                web: Some(ConstStr::new("camera")),
            },
            PermissionKind::Location(LocationPrecision::Fine) => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.ACCESS_FINE_LOCATION")),
                ios: Some(ConstStr::new(
                    "NSLocationAlwaysAndWhenInUseUsageDescription",
                )),
                macos: Some(ConstStr::new("NSLocationUsageDescription")),
                windows: Some(ConstStr::new("location")),
                linux: None,
                web: Some(ConstStr::new("geolocation")),
            },
            PermissionKind::Location(LocationPrecision::Coarse) => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.ACCESS_COARSE_LOCATION")),
                ios: Some(ConstStr::new("NSLocationWhenInUseUsageDescription")),
                macos: Some(ConstStr::new("NSLocationUsageDescription")),
                windows: Some(ConstStr::new("location")),
                linux: None,
                web: Some(ConstStr::new("geolocation")),
            },
            PermissionKind::Microphone => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.RECORD_AUDIO")),
                ios: Some(ConstStr::new("NSMicrophoneUsageDescription")),
                macos: Some(ConstStr::new("NSMicrophoneUsageDescription")),
                windows: Some(ConstStr::new("microphone")),
                linux: None,
                web: Some(ConstStr::new("microphone")),
            },
            PermissionKind::PhotoLibrary => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_EXTERNAL_STORAGE")),
                ios: Some(ConstStr::new("NSPhotoLibraryUsageDescription")),
                macos: Some(ConstStr::new("NSPhotoLibraryUsageDescription")),
                windows: Some(ConstStr::new("broadFileSystemAccess")),
                linux: None,
                web: Some(ConstStr::new("clipboard-read")),
            },
            PermissionKind::Contacts => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_CONTACTS")),
                ios: Some(ConstStr::new("NSContactsUsageDescription")),
                macos: Some(ConstStr::new("NSContactsUsageDescription")),
                windows: Some(ConstStr::new("contacts")),
                linux: None,
                web: Some(ConstStr::new("contacts")),
            },
            PermissionKind::Calendar => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_CALENDAR")),
                ios: Some(ConstStr::new("NSCalendarsUsageDescription")),
                macos: Some(ConstStr::new("NSCalendarsUsageDescription")),
                windows: Some(ConstStr::new("appointments")),
                linux: None,
                web: Some(ConstStr::new("calendar")),
            },
            PermissionKind::Bluetooth => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.BLUETOOTH_CONNECT")),
                ios: Some(ConstStr::new("NSBluetoothAlwaysUsageDescription")),
                macos: Some(ConstStr::new("NSBluetoothAlwaysUsageDescription")),
                windows: Some(ConstStr::new("bluetooth")),
                linux: None,
                web: Some(ConstStr::new("bluetooth")),
            },
            PermissionKind::Notifications => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.POST_NOTIFICATIONS")),
                ios: None,     // Runtime request only
                macos: None,   // Runtime request only
                windows: None, // No permission required
                linux: None,   // No permission required
                web: Some(ConstStr::new("notifications")),
            },
            PermissionKind::FileSystem => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_EXTERNAL_STORAGE")),
                ios: Some(ConstStr::new("NSPhotoLibraryUsageDescription")),
                macos: Some(ConstStr::new(
                    "com.apple.security.files.user-selected.read-write",
                )),
                windows: Some(ConstStr::new("broadFileSystemAccess")),
                linux: None,
                web: Some(ConstStr::new("file-system-access")),
            },
            PermissionKind::Network => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.INTERNET")),
                ios: None,   // No explicit permission
                macos: None, // Outgoing connections allowed
                windows: Some(ConstStr::new("internetClient")),
                linux: None, // No permission required
                web: None,   // CORS restrictions apply
            },
            PermissionKind::Sms => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_SMS")),
                ios: None,
                macos: None,
                windows: None,
                linux: None,
                web: None,
            },
            PermissionKind::PhoneState => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.READ_PHONE_STATE")),
                ios: None,
                macos: None,
                windows: None,
                linux: None,
                web: None,
            },
            PermissionKind::PhoneCall => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.CALL_PHONE")),
                ios: None,
                macos: None,
                windows: Some(ConstStr::new("phoneCall")),
                linux: None,
                web: None,
            },
            PermissionKind::SystemAlertWindow => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.SYSTEM_ALERT_WINDOW")),
                ios: None,
                macos: None,
                windows: None,
                linux: None,
                web: None,
            },
            PermissionKind::UserTracking => PlatformIdentifiers {
                android: None,
                ios: Some(ConstStr::new("NSUserTrackingUsageDescription")),
                macos: Some(ConstStr::new("NSUserTrackingUsageDescription")),
                windows: None,
                linux: None,
                web: Some(ConstStr::new("user-tracking")),
            },
            PermissionKind::FaceId => PlatformIdentifiers {
                android: None,
                ios: Some(ConstStr::new("NSFaceIDUsageDescription")),
                macos: Some(ConstStr::new("NSFaceIDUsageDescription")),
                windows: None,
                linux: None,
                web: None,
            },
            PermissionKind::LocalNetwork => PlatformIdentifiers {
                android: None,
                ios: Some(ConstStr::new("NSLocalNetworkUsageDescription")),
                macos: Some(ConstStr::new("NSLocalNetworkUsageDescription")),
                windows: None,
                linux: None,
                web: None,
            },
            PermissionKind::Appointments => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: None,
                windows: Some(ConstStr::new("appointments")),
                linux: None,
                web: None,
            },
            PermissionKind::WindowsPhoneCall => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: None,
                windows: Some(ConstStr::new("phoneCall")),
                linux: None,
                web: None,
            },
            PermissionKind::EnterpriseAuth => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: None,
                windows: Some(ConstStr::new("enterpriseAuthentication")),
                linux: None,
                web: None,
            },
            PermissionKind::Clipboard => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: None,
                windows: None,
                linux: None,
                web: Some(ConstStr::new("clipboard-read")),
            },
            PermissionKind::Payment => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: None,
                windows: None,
                linux: None,
                web: Some(ConstStr::new("payment-handler")),
            },
            PermissionKind::ScreenWakeLock => PlatformIdentifiers {
                android: None,
                ios: None,
                macos: None,
                windows: None,
                linux: None,
                web: Some(ConstStr::new("screen-wake-lock")),
            },
            PermissionKind::Custom {
                android,
                ios,
                macos,
                windows,
                linux,
                web,
            } => PlatformIdentifiers {
                android: Some(*android),
                ios: Some(*ios),
                macos: Some(*macos),
                windows: Some(*windows),
                linux: Some(*linux),
                web: Some(*web),
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
        if identifiers.windows.is_some() {
            flags = flags.with_platform(Platform::Windows);
        }
        if identifiers.linux.is_some() {
            flags = flags.with_platform(Platform::Linux);
        }
        if identifiers.web.is_some() {
            flags = flags.with_platform(Platform::Web);
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
    pub windows: Option<ConstStr>,
    pub linux: Option<ConstStr>,
    pub web: Option<ConstStr>,
}

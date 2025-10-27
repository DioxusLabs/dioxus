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
/// 
/// Only tested and verified permissions are included. For untested permissions,
/// use the `Custom` variant with platform-specific identifiers.
#[repr(C, u8)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SerializeConst)]
pub enum PermissionKind {
    /// Camera access - tested across all platforms
    Camera,
    /// Location access with precision - tested across all platforms
    Location(LocationPrecision),
    /// Microphone access - tested across all platforms  
    Microphone,
    /// Push notifications - tested on Android and Web
    Notifications,
    /// Custom permission with platform-specific identifiers for extensibility
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
            PermissionKind::Notifications => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.POST_NOTIFICATIONS")),
                ios: None,     // Runtime request only
                macos: None,   // Runtime request only
                windows: None, // No permission required
                linux: None,   // No permission required
                web: Some(ConstStr::new("notifications")),
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

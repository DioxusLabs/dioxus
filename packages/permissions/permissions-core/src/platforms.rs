use const_serialize::{ConstStr, SerializeConst};

/// Platform categories for permission mapping
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeConst)]
pub enum Platform {
    /// Mobile platforms
    Android,
    Ios,
    /// Desktop Darwin platform
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SerializeConst)]
#[allow(clippy::large_enum_variant)] // Custom variant contains large ConstStr fields needed for const serialization
pub enum PermissionKind {
    /// Camera access
    Camera,
    /// Location access with precision
    Location(LocationPrecision),
    /// Microphone access
    Microphone,
    /// Push notifications
    Notifications,
    /// Custom permission with platform-specific identifiers
    Custom {
        android: ConstStr,
        ios: ConstStr,
        macos: ConstStr,
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
            },
            PermissionKind::Location(LocationPrecision::Fine) => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.ACCESS_FINE_LOCATION")),
                ios: Some(ConstStr::new(
                    "NSLocationAlwaysAndWhenInUseUsageDescription",
                )),
                macos: Some(ConstStr::new("NSLocationUsageDescription")),
            },
            PermissionKind::Location(LocationPrecision::Coarse) => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.ACCESS_COARSE_LOCATION")),
                ios: Some(ConstStr::new("NSLocationWhenInUseUsageDescription")),
                macos: Some(ConstStr::new("NSLocationUsageDescription")),
            },
            PermissionKind::Microphone => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.RECORD_AUDIO")),
                ios: Some(ConstStr::new("NSMicrophoneUsageDescription")),
                macos: Some(ConstStr::new("NSMicrophoneUsageDescription")),
            },
            PermissionKind::Notifications => PlatformIdentifiers {
                android: Some(ConstStr::new("android.permission.POST_NOTIFICATIONS")),
                ios: None,   // Runtime request only
                macos: None, // Runtime request only
            },
            PermissionKind::Custom {
                android,
                ios,
                macos,
            } => PlatformIdentifiers {
                android: Some(*android),
                ios: Some(*ios),
                macos: Some(*macos),
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

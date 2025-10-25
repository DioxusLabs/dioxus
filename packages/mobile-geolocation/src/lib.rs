//! Cross-platform geolocation for Dioxus mobile apps
//!
//! This crate provides geolocation functionality for Android and iOS platforms
//! using clean, direct bindings without external build tools. Android uses JNI
//! with a single Java file compiled to DEX, while iOS uses objc2 for direct
//! Objective-C bindings. Permissions are automatically embedded via linker symbols
//! and injected into platform manifests by the Dioxus CLI.
//!
//! ## Features
//!
//! - `location-coarse`: Request coarse location permission (default)
//! - `location-fine`: Request fine/precise location permission
//! - `background-location`: Request background location access
//!
//! ## Usage
//!
//! ```rust,no_run
//! use dioxus_mobile_geolocation::last_known_location;
//!
//! if let Some((lat, lon)) = last_known_location() {
//!     println!("Location: {}, {}", lat, lon);
//! }
//! ```
//!
//! ## Permissions
//!
//! This crate uses the linker-based permission system. When you enable
//! `location-coarse` or `location-fine` features, the appropriate permissions
//! are embedded as linker symbols. The Dioxus CLI will automatically:
//!
//! - Add `<uses-permission>` entries to AndroidManifest.xml
//! - Add Info.plist keys to iOS/macOS bundles
//!
//! No manual manifest editing required!

mod error;
mod sys;

use permissions::{permission, Permission};

pub use error::{Error, Result};

/// Represents a geographic coordinate
#[derive(Debug, Clone, Copy)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

// Embed location permissions as linker symbols when features are enabled
#[cfg(feature = "location-fine")]
pub const LOCATION_FINE: Permission = permission!(
    Location(Fine),
    description = "Precise location for geolocation features"
);

#[cfg(feature = "location-coarse")]
pub const LOCATION_COARSE: Permission = permission!(
    Location(Coarse),
    description = "Approximate location for geolocation features"
);

// Optional background location (Android + iOS)
#[cfg(feature = "background-location")]
pub const BACKGROUND_LOCATION: Permission = permission!(
    Custom {
        android = "android.permission.ACCESS_BACKGROUND_LOCATION",
        ios = "NSLocationAlwaysAndWhenInUseUsageDescription",
        macos = "NSLocationUsageDescription",
        windows = "location",
        linux = "",
        web = ""
    },
    description = "Background location access"
);

/// Internal function to ensure permission constants are linked into the binary.
/// This prevents the linker from optimizing them away as dead code.
/// DO NOT REMOVE - this is required for the permission system to work.
#[doc(hidden)]
#[inline(never)]
pub fn __ensure_permissions_linked() {
    #[cfg(feature = "location-fine")]
    {
        let _ = &LOCATION_FINE;
    }
    #[cfg(feature = "location-coarse")]
    {
        let _ = &LOCATION_COARSE;
    }
    #[cfg(feature = "background-location")]
    {
        let _ = &BACKGROUND_LOCATION;
    }
}

/// Request location permissions at runtime.
///
/// This function triggers the system permission dialog for location access.
/// Returns `true` if the permission request was sent successfully, `false` otherwise.
///
/// ## Platform behavior
///
/// - **Android**: Calls `ActivityCompat.requestPermissions()` via JNI
/// - **iOS**: Calls `CLLocationManager.requestWhenInUseAuthorization()` via objc2
/// - **Other platforms**: Always returns `false`
///
/// ## Usage
///
/// Call this function before `last_known_location()` to ensure permissions are granted.
/// The user will see a system dialog asking for location permission.
pub fn request_location_permission() -> bool {
    // Ensure permissions are linked (prevents dead code elimination)
    __ensure_permissions_linked();

    sys::request_permission()
}

/// Get the last known location from the device.
///
/// Returns `Some((latitude, longitude))` if a location is available,
/// or `None` if no location has been cached or permissions are denied.
///
/// ## Platform behavior
///
/// - **Android**: Queries `LocationManager.getLastKnownLocation()` via JNI
/// - **iOS**: Queries `CLLocationManager.location` via objc2
/// - **Other platforms**: Always returns `None`
///
/// ## Permissions
///
/// This function requires location permissions to be granted at runtime.
/// The compile-time permissions are automatically embedded when you enable
/// the `location-coarse` or `location-fine` features.
///
/// On Android, you should request permissions using `request_location_permission()`
/// before calling this function.
///
/// On iOS, permissions are handled via Info.plist configuration.
pub fn last_known_location() -> Option<(f64, f64)> {
    // Ensure permissions are linked (prevents dead code elimination)
    __ensure_permissions_linked();

    sys::last_known()
}

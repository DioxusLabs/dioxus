// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

/// Permission state for geolocation permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum PermissionState {
    /// Permission not yet determined (user hasn't been asked)
    #[default]
    Prompt,

    /// Permission prompt shown with rationale (Android 12+)
    PromptWithRationale,

    /// Permission granted
    Granted,

    /// Permission denied
    Denied,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    /// Permission state for the location alias.
    ///
    /// On Android it requests/checks both ACCESS_COARSE_LOCATION and ACCESS_FINE_LOCATION permissions.
    ///
    /// On iOS it requests/checks location permissions.
    pub location: PermissionState,

    /// Permissions state for the coarseLocation alias.
    ///
    /// On Android it requests/checks ACCESS_COARSE_LOCATION.
    ///
    /// On Android 12+, users can choose between Approximate location (ACCESS_COARSE_LOCATION) and Precise location (ACCESS_FINE_LOCATION).
    ///
    /// On iOS it will have the same value as the `location` alias.
    pub coarse_location: PermissionState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionOptions {
    /// High accuracy mode (such as GPS, if available)
    /// Will be ignored on Android 12+ if users didn't grant the ACCESS_FINE_LOCATION permission.
    pub enable_high_accuracy: bool,
    /// The maximum wait time in milliseconds for location updates.
    /// Default: 10000
    /// On Android the timeout gets ignored for getCurrentPosition.
    /// Ignored on iOS.
    // TODO: Handle Infinity and default to it.
    // TODO: Should be u64+ but specta doesn't like that?
    pub timeout: u32,
    /// The maximum age in milliseconds of a possible cached position that is acceptable to return.
    /// Default: 0
    /// Ignored on iOS.
    // TODO: Handle Infinity.
    // TODO: Should be u64+ but specta doesn't like that?
    pub maximum_age: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionType {
    Location,
    CoarseLocation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coordinates {
    /// Latitude in decimal degrees.
    pub latitude: f64,
    /// Longitude in decimal degrees.
    pub longitude: f64,
    /// Accuracy level of the latitude and longitude coordinates in meters.
    pub accuracy: f64,
    /// Accuracy level of the altitude coordinate in meters, if available.
    /// Available on all iOS versions and on Android 8 and above.
    pub altitude_accuracy: Option<f64>,
    /// The altitude the user is at, if available.
    pub altitude: Option<f64>,
    // The speed the user is traveling, if available.
    pub speed: Option<f64>,
    /// The heading the user is facing, if available.
    pub heading: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    /// Creation time for these coordinates.
    // TODO: Check if we're actually losing precision.
    pub timestamp: u64,
    /// The GPS coordinates along with the accuracy of the data.
    pub coords: Coordinates,
}

// =============================================================================
// Live Activity types (iOS 16.1+)
// =============================================================================

/// Result from starting a Live Activity
#[cfg(target_os = "ios")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveActivityResult {
    /// Unique identifier for the activity
    pub activity_id: String,
    /// Current latitude displayed in the activity
    pub latitude: f64,
    /// Current longitude displayed in the activity
    pub longitude: f64,
    /// Horizontal accuracy in meters
    pub accuracy: f64,
}

/// Result from updating a Live Activity
#[cfg(target_os = "ios")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveActivityUpdate {
    /// Current latitude after update
    pub latitude: f64,
    /// Current longitude after update
    pub longitude: f64,
    /// Horizontal accuracy in meters
    pub accuracy: f64,
}

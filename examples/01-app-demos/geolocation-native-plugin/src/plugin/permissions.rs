// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Geolocation permissions declaration using Dioxus permissions system
//!
//! This module declares the permissions required for geolocation functionality.
//! These permissions are embedded in the binary and can be extracted by build tools
//! to inject into platform-specific configuration files.

use manganis::permissions;
use manganis::permissions::{static_permission, LocationPrecision, Permission, PermissionBuilder};

/// Fine location permission
///
/// This permission allows the app to access precise location data using GPS.
/// On Android, this corresponds to `ACCESS_FINE_LOCATION`.
/// On iOS, this corresponds to `NSLocationWhenInUseUsageDescription`.
pub const FINE_LOCATION: Permission =
    static_permission!(PermissionBuilder::location(LocationPrecision::Fine)
        .with_description("Access your precise location to provide location-based services")
        .build());

/// Coarse location permission
///
/// This permission allows the app to access approximate location data.
/// On Android, this corresponds to `ACCESS_COARSE_LOCATION`.
/// On iOS, this corresponds to `NSLocationWhenInUseUsageDescription`.
pub const COARSE_LOCATION: Permission =
    static_permission!(PermissionBuilder::location(LocationPrecision::Coarse)
        .with_description("Access your approximate location to provide location-based services")
        .build());

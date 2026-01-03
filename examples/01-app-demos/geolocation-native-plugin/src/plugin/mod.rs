// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Dioxus Geolocation Plugin
//!
//! This plugin provides APIs for getting and tracking the device's current position
//! on Android and iOS mobile platforms.
//!
//! This example demonstrates the use of the `#[manganis::ffi]` macro for automatic
//! FFI binding generation between Rust and native platforms.

pub use models::*;

mod error;
mod models;

#[cfg(any(target_os = "android", target_os = "ios"))]
mod permissions;

pub use error::{Error, Result};

// =============================================================================
// iOS FFI Bridge using #[manganis::ffi]
// Only enabled on actual iOS target (not macOS desktop)
// =============================================================================

/// iOS/macOS native bindings - the macro generates all FFI code automatically.
/// The path "src/ios" points to the SwiftPM package containing GeolocationPlugin.swift
#[cfg(all(feature = "metadata", any(target_os = "ios", target_os = "macos")))]
#[manganis::ffi("src/ios")]
extern "Swift" {
    /// The native GeolocationPlugin class
    pub type GeolocationPlugin;

    /// Get current position as JSON string
    /// Swift signature: func getCurrentPositionJson(_ optionsJson: String) -> String
    pub fn get_current_position_json(this: &GeolocationPlugin, options_json: String) -> String;

    /// Check permissions and return status as JSON
    /// Swift signature: func checkPermissionsJson() -> String
    pub fn check_permissions_json(this: &GeolocationPlugin) -> String;

    /// Request permissions with optional types list as JSON, return status as JSON
    /// Swift signature: func requestPermissionsJson(_ permissionsJson: String) -> String
    pub fn request_permissions_json(this: &GeolocationPlugin, permissions_json: String) -> String;
}

// =============================================================================
// Android FFI Bridge using #[manganis::ffi]
// =============================================================================

/// Android native bindings - the macro generates all JNI code automatically.
/// The path "src/android" points to the Gradle project containing GeolocationPlugin.kt
#[cfg(all(feature = "metadata", target_os = "android"))]
#[manganis::ffi("src/android")]
extern "Kotlin" {
    /// The native GeolocationPlugin class
    pub type GeolocationPlugin;

    /// Get current position as JSON string
    /// Kotlin signature: fun getCurrentPositionJson(optionsJson: String): String
    pub fn get_current_position_json(this: &GeolocationPlugin, options_json: String) -> String;

    /// Check permissions and return status as JSON
    /// Kotlin signature: fun checkPermissionsJson(): String
    pub fn check_permissions_json(this: &GeolocationPlugin) -> String;

    /// Request permissions with optional types list as JSON, return status as JSON
    /// Kotlin signature: fun requestPermissionsJson(permissionsJson: String): String
    pub fn request_permissions_json(this: &GeolocationPlugin, permissions_json: String) -> String;
}

// =============================================================================
// Stub for non-native platforms (web, Linux desktop, etc.)
// =============================================================================

#[cfg(not(any(
    all(feature = "metadata", any(target_os = "ios", target_os = "macos")),
    all(feature = "metadata", target_os = "android")
)))]
pub struct GeolocationPlugin;

#[cfg(not(any(
    all(feature = "metadata", any(target_os = "ios", target_os = "macos")),
    all(feature = "metadata", target_os = "android")
)))]
impl GeolocationPlugin {
    pub fn new() -> Result<Self> {
        Err(Error::PlatformBridge(
            "Geolocation is only supported on Android, iOS, and macOS".to_string(),
        ))
    }
}

#[cfg(not(any(
    all(feature = "metadata", any(target_os = "ios", target_os = "macos")),
    all(feature = "metadata", target_os = "android")
)))]
fn get_current_position_json(_: &GeolocationPlugin, _: String) -> Result<String> {
    Err(Error::PlatformBridge(
        "Geolocation is only supported on Android, iOS, and macOS".to_string(),
    ))
}

#[cfg(not(any(
    all(feature = "metadata", any(target_os = "ios", target_os = "macos")),
    all(feature = "metadata", target_os = "android")
)))]
fn check_permissions_json(_: &GeolocationPlugin) -> Result<String> {
    Err(Error::PlatformBridge(
        "Geolocation is only supported on Android, iOS, and macOS".to_string(),
    ))
}

#[cfg(not(any(
    all(feature = "metadata", any(target_os = "ios", target_os = "macos")),
    all(feature = "metadata", target_os = "android")
)))]
fn request_permissions_json(_: &GeolocationPlugin, _: String) -> Result<String> {
    Err(Error::PlatformBridge(
        "Geolocation is only supported on Android, iOS, and macOS".to_string(),
    ))
}

// =============================================================================
// Unified Geolocation API
// =============================================================================

/// Access to the geolocation APIs.
///
/// This struct provides a unified interface for accessing geolocation functionality
/// on both Android and iOS platforms. It uses the `#[manganis::ffi]` macro for
/// automatic FFI binding generation.
///
/// # Example
///
/// ```rust,no_run
/// use plugin::{Geolocation, PermissionState, PositionOptions};
///
/// let mut geolocation = Geolocation::new();
///
/// // Check permissions
/// let status = geolocation.check_permissions()?;
/// if status.location == PermissionState::Prompt {
///     let new_status = geolocation.request_permissions(None)?;
/// }
///
/// // Get current position
/// let options = PositionOptions {
///     enable_high_accuracy: true,
///     timeout: 10000,
///     maximum_age: 0,
/// };
/// let position = geolocation.get_current_position(Some(options))?;
/// println!("Latitude: {}, Longitude: {}", position.coords.latitude, position.coords.longitude);
///
/// # Ok::<(), plugin::Error>(())
/// ```
pub struct Geolocation {
    plugin: Option<GeolocationPlugin>,
}

impl Geolocation {
    /// Create a new Geolocation instance
    pub fn new() -> Self {
        Self { plugin: None }
    }

    /// Get or initialize the plugin instance
    fn get_plugin(&mut self) -> Result<&GeolocationPlugin> {
        if self.plugin.is_none() {
            self.plugin = Some(GeolocationPlugin::new()?);
        }
        Ok(self.plugin.as_ref().unwrap())
    }

    /// Get the device's current position.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional position options. If `None`, default options are used.
    ///
    /// # Returns
    ///
    /// Returns the current position or an error if the location cannot be obtained.
    pub fn get_current_position(&mut self, options: Option<PositionOptions>) -> Result<Position> {
        let options = options.unwrap_or_default();
        let options_json = serde_json::to_string(&options).map_err(Error::Json)?;

        let plugin = self.get_plugin()?;
        let result_json = get_current_position_json(plugin, options_json)?;

        // Check for error in response
        let json_value: serde_json::Value =
            serde_json::from_str(&result_json).map_err(Error::Json)?;
        if let Some(error_msg) = json_value.get("error") {
            return Err(Error::LocationUnavailable(
                error_msg.as_str().unwrap_or("Unknown error").to_string(),
            ));
        }

        let position: Position = serde_json::from_str(&result_json).map_err(Error::Json)?;
        Ok(position)
    }

    /// Check the current permission status.
    ///
    /// # Returns
    ///
    /// Returns the permission status for location and coarse location permissions.
    pub fn check_permissions(&mut self) -> Result<PermissionStatus> {
        let plugin = self.get_plugin()?;
        let result_json = check_permissions_json(plugin)?;
        let status: PermissionStatus = serde_json::from_str(&result_json).map_err(Error::Json)?;
        Ok(status)
    }

    /// Request location permissions from the user.
    ///
    /// # Arguments
    ///
    /// * `permissions` - Optional list of specific permission types to request.
    ///   If `None`, requests all location permissions.
    ///
    /// # Returns
    ///
    /// Returns the permission status after the user responds to the permission request.
    pub fn request_permissions(
        &mut self,
        permissions: Option<Vec<PermissionType>>,
    ) -> Result<PermissionStatus> {
        let perms_json = serde_json::to_string(&permissions).map_err(Error::Json)?;
        let plugin = self.get_plugin()?;
        let result_json = request_permissions_json(plugin, perms_json)?;
        let status: PermissionStatus = serde_json::from_str(&result_json).map_err(Error::Json)?;
        Ok(status)
    }
}

impl Default for Geolocation {
    fn default() -> Self {
        Self::new()
    }
}

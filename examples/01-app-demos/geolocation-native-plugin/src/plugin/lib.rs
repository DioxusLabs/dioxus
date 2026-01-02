// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Dioxus Geolocation Plugin
//!
//! This plugin provides APIs for getting and tracking the device's current position
//! on Android and iOS mobile platforms.

pub use models::*;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

mod error;
mod models;

#[cfg(any(target_os = "android", target_os = "ios"))]
mod permissions;

// Declare Android artifacts for automatic bundling
#[cfg(all(feature = "metadata", target_os = "android"))]
dioxus_platform_bridge::android_plugin!(
    plugin = "geolocation",
    aar = { env = "DIOXUS_ANDROID_ARTIFACT" },
    deps = ["implementation(\"com.google.android.gms:play-services-location:21.3.0\")"]
);

// Declare iOS/macOS Swift sources for automatic bundling
#[cfg(all(feature = "metadata", any(target_os = "ios", target_os = "macos")))]
dioxus_platform_bridge::ios_plugin!(
    plugin = "geolocation",
    spm = { path = "ios", product = "GeolocationPlugin" }
);

pub use error::{Error, Result};

#[cfg(target_os = "android")]
use android::Geolocation as PlatformGeolocation;
#[cfg(target_os = "ios")]
use ios::Geolocation as PlatformGeolocation;

/// Access to the geolocation APIs.
///
/// This struct provides a unified interface for accessing geolocation functionality
/// on both Android and iOS platforms. It automatically initializes and manages the
/// platform-specific implementations.
///
/// # Example
///
/// ```rust,no_run
/// use dioxus_geolocation::{Geolocation, PermissionState, PositionOptions};
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
/// # Ok::<(), dioxus_geolocation::Error>(())
/// ```
pub struct Geolocation {
    #[cfg(target_os = "android")]
    inner: android::Geolocation,
    #[cfg(target_os = "ios")]
    inner: ios::Geolocation,
}

impl Geolocation {
    /// Create a new Geolocation instance
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "android")]
            inner: android::Geolocation::new(),
            #[cfg(target_os = "ios")]
            inner: ios::Geolocation::new(),
        }
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
        #[cfg(target_os = "android")]
        {
            self.inner.get_current_position(options)
        }
        #[cfg(target_os = "ios")]
        {
            (&self.inner).get_current_position(options)
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let _ = options;
            Err(Error::PlatformBridge(
                "Geolocation is only supported on Android and iOS".to_string(),
            ))
        }
    }

    /// Check the current permission status.
    ///
    /// # Returns
    ///
    /// Returns the permission status for location and coarse location permissions.
    pub fn check_permissions(&mut self) -> Result<PermissionStatus> {
        #[cfg(target_os = "android")]
        {
            self.inner.check_permissions()
        }
        #[cfg(target_os = "ios")]
        {
            (&self.inner).check_permissions()
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            Err(Error::PlatformBridge(
                "Geolocation is only supported on Android and iOS".to_string(),
            ))
        }
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
        #[cfg(target_os = "android")]
        {
            self.inner.request_permissions(permissions)
        }
        #[cfg(target_os = "ios")]
        {
            (&self.inner).request_permissions(permissions)
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let _ = permissions;
            Err(Error::PlatformBridge(
                "Geolocation is only supported on Android and iOS".to_string(),
            ))
        }
    }
}

impl Default for Geolocation {
    fn default() -> Self {
        Self::new()
    }
}

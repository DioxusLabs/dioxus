// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use dioxus_platform_bridge::darwin::MainThreadCell;
use objc2::{msg_send, MainThreadMarker};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::error::{Error, Result};
use crate::models::*;

extern "C" {
    fn dioxus_geolocation_plugin_init();
}

/// iOS implementation of the geolocation API
pub struct Geolocation {
    plugin_instance: MainThreadCell<*mut objc2::runtime::Object>,
}

unsafe impl Send for Geolocation {}
unsafe impl Sync for Geolocation {}

impl Geolocation {
    /// Create a new Geolocation instance
    pub fn new() -> Self {
        unsafe {
            // Ensure the Swift static library is linked and the class is registered
            dioxus_geolocation_plugin_init();
        }

        Self {
            plugin_instance: MainThreadCell::new(),
        }
    }

    /// Get or initialize the plugin instance
    fn get_plugin_instance(&self, mtm: MainThreadMarker) -> Result<&mut objc2::runtime::Object> {
        unsafe {
            let ptr_ref = self.plugin_instance.get_or_try_init_with(mtm, || {
                let class_name =
                    CStr::from_bytes_with_nul(b"GeolocationPlugin\0").expect("Invalid class name");
                let class = objc2::runtime::Class::get(class_name).ok_or_else(|| {
                    Error::Ios(
                        "GeolocationPlugin class not found. Ensure the Swift package is built and linked."
                            .to_string(),
                    )
                })?;

                let instance: *mut objc2::runtime::Object = msg_send![class, alloc];
                let instance: *mut objc2::runtime::Object = msg_send![instance, init];
                Ok::<*mut objc2::runtime::Object, Error>(instance)
            })?;

            Ok(&mut **ptr_ref)
        }
    }

    /// Get current position
    pub fn get_current_position(&self, options: Option<PositionOptions>) -> Result<Position> {
        let options = options.unwrap_or_default();
        let mtm =
            MainThreadMarker::new().ok_or_else(|| Error::Ios("Not on main thread".to_string()))?;

        let plugin = self.get_plugin_instance(mtm)?;

        // Serialize options to JSON
        let options_json = serde_json::to_string(&options).map_err(|e| Error::Json(e))?;

        unsafe {
            // Create NSString from JSON using NSString::stringWithUTF8String: (class method)
            let json_cstr = CString::new(options_json)
                .map_err(|e| Error::Ios(format!("Invalid JSON string: {}", e)))?;
            let nsstring_class =
                objc2::runtime::Class::get(CStr::from_bytes_with_nul(b"NSString\0").unwrap())
                    .ok_or_else(|| Error::Ios("NSString class not found".to_string()))?;
            let json_nsstring: *mut objc2::runtime::Object =
                msg_send![nsstring_class, stringWithUTF8String: json_cstr.as_ptr()];

            // Call getCurrentPositionJson: on the plugin
            let result: *mut objc2::runtime::Object = msg_send![
                plugin,
                getCurrentPositionJson: json_nsstring
            ];

            // Convert NSString to Rust String using UTF8String method
            let result_cstr: *const c_char = msg_send![&mut *result, UTF8String];
            let result_str = CStr::from_ptr(result_cstr)
                .to_str()
                .map_err(|e| Error::Ios(format!("Invalid UTF-8 in result: {}", e)))?;

            // Deserialize JSON to Position
            let position: Position =
                serde_json::from_str(result_str).map_err(|e| Error::Json(e))?;

            Ok(position)
        }
    }

    /// Check permissions
    pub fn check_permissions(&self) -> Result<PermissionStatus> {
        let mtm =
            MainThreadMarker::new().ok_or_else(|| Error::Ios("Not on main thread".to_string()))?;

        let plugin = self.get_plugin_instance(mtm)?;

        unsafe {
            // Call checkPermissionsJson on the plugin
            let result: *mut objc2::runtime::Object = msg_send![plugin, checkPermissionsJson];

            // Convert NSString to Rust String
            let result_cstr: *const c_char = msg_send![&mut *result, UTF8String];
            let result_str = CStr::from_ptr(result_cstr)
                .to_str()
                .map_err(|e| Error::Ios(format!("Invalid UTF-8 in result: {}", e)))?;
            let status: PermissionStatus =
                serde_json::from_str(result_str).map_err(|e| Error::Json(e))?;

            Ok(status)
        }
    }

    /// Request permissions
    pub fn request_permissions(
        &self,
        permissions: Option<Vec<PermissionType>>,
    ) -> Result<PermissionStatus> {
        let mtm =
            MainThreadMarker::new().ok_or_else(|| Error::Ios("Not on main thread".to_string()))?;

        let plugin = self.get_plugin_instance(mtm)?;

        // Serialize permissions to JSON
        let perms_json = serde_json::to_string(&permissions).map_err(|e| Error::Json(e))?;

        unsafe {
            // Create NSString from JSON
            let json_cstr = CString::new(perms_json)
                .map_err(|e| Error::Ios(format!("Invalid JSON string: {}", e)))?;
            let nsstring_class =
                objc2::runtime::Class::get(CStr::from_bytes_with_nul(b"NSString\0").unwrap())
                    .ok_or_else(|| Error::Ios("NSString class not found".to_string()))?;
            let json_nsstring: *mut objc2::runtime::Object =
                msg_send![nsstring_class, stringWithUTF8String: json_cstr.as_ptr()];

            // Call requestPermissionsJson: on the plugin
            let result: *mut objc2::runtime::Object = msg_send![
                plugin,
                requestPermissionsJson: json_nsstring
            ];

            // Convert NSString to Rust String
            let result_cstr: *const c_char = msg_send![&mut *result, UTF8String];
            let result_str = CStr::from_ptr(result_cstr)
                .to_str()
                .map_err(|e| Error::Ios(format!("Invalid UTF-8 in result: {}", e)))?;
            let status: PermissionStatus =
                serde_json::from_str(result_str).map_err(|e| Error::Json(e))?;

            Ok(status)
        }
    }
}

impl Default for Geolocation {
    fn default() -> Self {
        Self::new()
    }
}

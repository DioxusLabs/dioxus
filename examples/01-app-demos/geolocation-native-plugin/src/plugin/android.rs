// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use dioxus_platform_bridge::android::with_activity;
use jni::{
    objects::{GlobalRef, JObject, JString, JValue},
    JNIEnv,
};
use serde_json::Value as JsonValue;

use crate::error::{Error, Result};
use crate::models::*;

const PLUGIN_CLASS: &str = "app/tauri/geolocation/GeolocationPlugin";

/// Android implementation of the geolocation API
pub struct Geolocation {
    plugin_instance: Option<GlobalRef>,
}

impl Geolocation {
    /// Create a new Geolocation instance
    pub fn new() -> Self {
        Self {
            plugin_instance: None,
        }
    }

    /// Initialize the plugin and get an instance
    fn get_plugin_instance(&mut self, env: &mut JNIEnv) -> Result<GlobalRef> {
        if let Some(ref instance) = self.plugin_instance {
            Ok(instance.clone())
        } else {
            let _plugin_class = env.find_class(PLUGIN_CLASS)?;

            // Create a new instance - we need to get the activity first
            let instance = with_activity(|env, activity| {
                // Call constructor: GeolocationPlugin(Activity)
                let plugin_obj = env
                    .new_object(
                        PLUGIN_CLASS,
                        "(Landroid/app/Activity;)V",
                        &[JValue::Object(activity)],
                    )
                    .ok()?;

                // Call load method with null WebView for now (not needed for Dioxus)
                let null_webview = JObject::null();
                env.call_method(
                    &plugin_obj,
                    "load",
                    "(Landroid/webkit/WebView;)V",
                    &[JValue::Object(&null_webview)],
                )
                .ok()?;

                Some(env.new_global_ref(&plugin_obj).ok()?)
            })
            .ok_or_else(|| Error::PlatformBridge("Failed to create plugin instance".to_string()))?;

            self.plugin_instance = Some(instance.clone());
            Ok(instance)
        }
    }

    /// Get current position
    pub fn get_current_position(&mut self, options: Option<PositionOptions>) -> Result<Position> {
        let options = options.unwrap_or_default();

        with_activity(|env, _activity| {
            let plugin = self.get_plugin_instance(env).ok()?;

            // Create a Java HashMap with the options
            let options_map = env.new_object("java/util/HashMap", "()V", &[]).ok()?;

            // Put enableHighAccuracy
            let key_acc = env.new_string("enableHighAccuracy").ok()?;
            // Create java.lang.Boolean from Rust bool
            let val_acc_obj = env
                .call_static_method(
                    "java/lang/Boolean",
                    "valueOf",
                    "(Z)Ljava/lang/Boolean;",
                    &[JValue::Bool(if options.enable_high_accuracy {
                        1
                    } else {
                        0
                    })],
                )
                .ok()?
                .l()
                .ok()?;
            env.call_method(
                &options_map,
                "put",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                &[JValue::Object(&key_acc), JValue::Object(&val_acc_obj)],
            )
            .ok()?;

            // Put maximumAge
            let key_age = env.new_string("maximumAge").ok()?;
            let val_age_obj = env
                .call_static_method(
                    "java/lang/Long",
                    "valueOf",
                    "(J)Ljava/lang/Long;",
                    &[JValue::Long(options.maximum_age as i64)],
                )
                .ok()?
                .l()
                .ok()?;
            env.call_method(
                &options_map,
                "put",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                &[JValue::Object(&key_age), JValue::Object(&val_age_obj)],
            )
            .ok()?;

            // Put timeout
            let key_timeout = env.new_string("timeout").ok()?;
            let val_timeout_obj = env
                .call_static_method(
                    "java/lang/Long",
                    "valueOf",
                    "(J)Ljava/lang/Long;",
                    &[JValue::Long(options.timeout as i64)],
                )
                .ok()?
                .l()
                .ok()?;
            env.call_method(
                &options_map,
                "put",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                &[
                    JValue::Object(&key_timeout),
                    JValue::Object(&val_timeout_obj),
                ],
            )
            .ok()?;

            // Call getCurrentPositionJson(Map<String, Object>): String
            let result = env
                .call_method(
                    &plugin,
                    "getCurrentPositionJson",
                    "(Ljava/util/Map;)Ljava/lang/String;",
                    &[JValue::Object(&options_map)],
                )
                .ok()?;

            let jstr_obj = result.l().ok()?;
            let jstr: JString = JString::from(jstr_obj);
            let result_string: String = env.get_string(&jstr).ok()?.into();

            // Deserialize the JSON result
            let json_value: JsonValue = serde_json::from_str(&result_string).ok()?;

            // Check if it's an error
            if let Some(error_msg) = json_value.get("error") {
                return Some(Err(Error::LocationUnavailable(
                    error_msg.as_str().unwrap_or("Unknown error").to_string(),
                )));
            }

            let position: Position = serde_json::from_value(json_value).ok()?;
            Some(Ok(position))
        })
        .ok_or_else(|| Error::PlatformBridge("Failed to get current position".to_string()))?
    }

    /// Check permissions
    pub fn check_permissions(&mut self) -> Result<PermissionStatus> {
        with_activity(|env, _activity| {
            let plugin = self.get_plugin_instance(env).ok()?;

            let result = env
                .call_method(&plugin, "checkPermissionsJson", "()Ljava/lang/String;", &[])
                .ok()?;

            let jstr_obj = result.l().ok()?;
            let jstr: JString = JString::from(jstr_obj);
            let result_string: String = env.get_string(&jstr).ok()?.into();

            let status: PermissionStatus = serde_json::from_str(&result_string).ok()?;
            Some(Ok(status))
        })
        .ok_or_else(|| Error::PlatformBridge("Failed to check permissions".to_string()))?
    }

    /// Request permissions
    pub fn request_permissions(
        &mut self,
        permissions: Option<Vec<PermissionType>>,
    ) -> Result<PermissionStatus> {
        with_activity(|env, _activity| {
            let plugin = self.get_plugin_instance(env).ok()?;

            // Serialize permissions to JSON
            let perms_json = serde_json::to_string(&permissions).ok()?;
            let perms_string = env.new_string(&perms_json).ok()?;

            let result = env
                .call_method(
                    &plugin,
                    "requestPermissionsJson",
                    "(Ljava/lang/String;)Ljava/lang/String;",
                    &[JValue::Object(&perms_string)],
                )
                .ok()?;

            let jstr_obj = result.l().ok()?;
            let jstr: JString = JString::from(jstr_obj);
            let result_string: String = env.get_string(&jstr).ok()?.into();

            let status: PermissionStatus = serde_json::from_str(&result_string).ok()?;
            Some(Ok(status))
        })
        .ok_or_else(|| Error::PlatformBridge("Failed to request permissions".to_string()))?
    }
}

impl Default for Geolocation {
    fn default() -> Self {
        Self::new()
    }
}

// use std::{env, path::PathBuf};

// use dioxus_mobile_plugin_build::{
//     build_android_library, build_swift_package, AndroidLibraryConfig, SwiftPackageConfig,
// };

// const SWIFT_PRODUCT: &str = "GeolocationPlugin";
// const SWIFT_MIN_IOS: &str = "13.0";
// const ANDROID_AAR_PREFERRED: &str = "android/build/outputs/aar/geolocation-plugin-release.aar";

// fn main() {
//     println!("cargo:rerun-if-changed=ios/Package.swift");
//     println!("cargo:rerun-if-changed=ios/Sources/GeolocationPlugin.swift");
//     println!("cargo:rerun-if-changed=android/build.gradle.kts");
//     println!("cargo:rerun-if-changed=android/settings.gradle.kts");
//     println!("cargo:rerun-if-changed=android/src");

//     let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
//     let swift_package_dir = manifest_dir.join("ios");
//     let android_project_dir = manifest_dir.join("android");
//     let preferred_aar = manifest_dir.join(ANDROID_AAR_PREFERRED);

//     if let Err(err) = build_swift_package(&SwiftPackageConfig {
//         product: SWIFT_PRODUCT,
//         min_ios_version: SWIFT_MIN_IOS,
//         package_dir: &swift_package_dir,
//         link_frameworks: &["CoreLocation", "Foundation"],
//         link_libraries: &[
//             "swiftCompatibility56",
//             "swiftCompatibilityConcurrency",
//             "swiftCompatibilityPacks",
//         ],
//     }) {
//         panic!("Failed to build Swift plugin: {err}");
//     }

//     if let Err(err) = build_android_library(&AndroidLibraryConfig {
//         project_dir: &android_project_dir,
//         preferred_artifact: &preferred_aar,
//         artifact_env_key: "DIOXUS_ANDROID_ARTIFACT",
//         gradle_task: "assembleRelease",
//     }) {
//         panic!("Failed to build Android plugin: {err}");
//     }
// }

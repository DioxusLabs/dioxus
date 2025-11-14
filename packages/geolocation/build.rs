use std::{env, path::PathBuf};

use dioxus_mobile_plugin_build::{
    build_android_library, build_swift_package, AndroidLibraryConfig, SwiftPackageConfig,
};

const SWIFT_PRODUCT: &str = "GeolocationPlugin";
const SWIFT_MIN_IOS: &str = "13.0";
const ANDROID_AAR_PREFERRED: &str = "android/build/outputs/aar/geolocation-plugin-release.aar";

fn main() {
    println!("cargo:rerun-if-changed=ios/Package.swift");
    println!("cargo:rerun-if-changed=ios/Sources/GeolocationPlugin.swift");
    println!("cargo:rerun-if-changed=android/build.gradle.kts");
    println!("cargo:rerun-if-changed=android/settings.gradle.kts");
    println!("cargo:rerun-if-changed=android/src");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let swift_package_dir = manifest_dir.join("ios");
    let android_project_dir = manifest_dir.join("android");
    let preferred_aar = manifest_dir.join(ANDROID_AAR_PREFERRED);

    if let Err(err) = build_swift_package(&SwiftPackageConfig {
        product: SWIFT_PRODUCT,
        min_ios_version: SWIFT_MIN_IOS,
        package_dir: &swift_package_dir,
        link_frameworks: &["CoreLocation", "Foundation"],
        link_libraries: &[
            "swiftCompatibility56",
            "swiftCompatibilityConcurrency",
            "swiftCompatibilityPacks",
        ],
    }) {
        panic!("Failed to build Swift plugin: {err}");
    }

    if let Err(err) = build_android_library(&AndroidLibraryConfig {
        project_dir: &android_project_dir,
        preferred_artifact: &preferred_aar,
        artifact_env_key: "DIOXUS_ANDROID_ARTIFACT",
        gradle_task: "assembleRelease",
    }) {
        panic!("Failed to build Android plugin: {err}");
    }
}

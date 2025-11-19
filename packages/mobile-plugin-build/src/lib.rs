//! Build-script helpers for Dioxus mobile plugins.
//!
//! This crate centralizes the shared Gradle (Android) and Swift Package (iOS)
//! build steps the plugins need so that each plugin crate can keep its
//! `build.rs` minimal.

use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Result alias used throughout the helper functions.
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Configuration for compiling and linking a Swift package for iOS targets.
pub struct SwiftPackageConfig<'a> {
    /// Name of the Swift product to build (must match the Package.swift product).
    pub product: &'a str,
    /// Minimum iOS version string, e.g. `"13.0"`.
    pub min_ios_version: &'a str,
    /// Absolute path to the Swift package directory (containing Package.swift).
    pub package_dir: &'a Path,
    /// Additional frameworks to link (passed as `cargo:rustc-link-lib=framework=...`).
    pub link_frameworks: &'a [&'a str],
    /// Extra static/dynamic libraries to link (passed as `cargo:rustc-link-lib=...`).
    pub link_libraries: &'a [&'a str],
}

/// Build the configured Swift package when targeting iOS and emit the linker
/// configuration required for Cargo to consume the produced static library.
pub fn build_swift_package(config: &SwiftPackageConfig<'_>) -> Result<()> {
    let target = env::var("TARGET")?;
    if !target.contains("apple-ios") {
        return Ok(());
    }

    let (swift_target, sdk_name) = swift_target_and_sdk(&target, config.min_ios_version)
        .ok_or_else(|| format!("Unsupported iOS target `{target}` for Swift compilation"))?;
    let sdk_path = lookup_sdk_path(sdk_name)?;

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let configuration = if profile == "release" {
        "release"
    } else {
        "debug"
    };

    let build_dir = PathBuf::from(env::var("OUT_DIR")?).join("swift-build");

    let status = Command::new("xcrun")
        .arg("swift")
        .arg("build")
        .arg("--package-path")
        .arg(config.package_dir)
        .arg("--configuration")
        .arg(configuration)
        .arg("--triple")
        .arg(&swift_target)
        .arg("--sdk")
        .arg(&sdk_path)
        .arg("--product")
        .arg(config.product)
        .arg("--build-path")
        .arg(&build_dir)
        .status()?;

    if !status.success() {
        return Err("swift build failed. Check the log above for details.".into());
    }

    let lib_path = find_static_lib(&build_dir, configuration, &swift_target, config.product)
        .ok_or_else(|| {
            format!(
                "Could not locate Swift static library for product `{}`",
                config.product
            )
        })?;

    if let Some(parent) = lib_path.parent() {
        println!("cargo:rustc-link-search=native={}", parent.display());
    }
    let runtime_lib_dir = swift_runtime_lib_dir(&swift_target)?;
    println!(
        "cargo:rustc-link-search=native={}",
        runtime_lib_dir.display()
    );
    println!("cargo:rustc-link-lib=static={}", config.product);
    // Force load the plugin archive so ObjC registries are included.
    println!("cargo:rustc-link-arg=-Xlinker");
    println!("cargo:rustc-link-arg=-force_load");
    println!("cargo:rustc-link-arg=-Xlinker");
    println!("cargo:rustc-link-arg={}", lib_path.display());
    println!("cargo:rustc-link-arg=-ObjC");

    for framework in config.link_frameworks {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
    for lib in config.link_libraries {
        println!("cargo:rustc-link-lib={lib}");
    }

    Ok(())
}

/// Configuration shared by Android plugin builds.
pub struct AndroidLibraryConfig<'a> {
    /// Absolute path to the Gradle project directory (contains `gradlew`/`build.gradle.kts`).
    pub project_dir: &'a Path,
    /// Preferred location of the built AAR (relative to the crate root).
    pub preferred_artifact: &'a Path,
    /// The environment variable name to expose the copied artifact path under.
    pub artifact_env_key: &'a str,
    /// The Gradle task to run when building (defaults to `assembleRelease` in users).
    pub gradle_task: &'a str,
}

/// Compile the Android library with Gradle when targeting Android and expose the
/// built AAR through the configured environment variable.
pub fn build_android_library(config: &AndroidLibraryConfig<'_>) -> Result<()> {
    let target = env::var("TARGET")?;
    if !target.contains("android") {
        return Ok(());
    }

    let gradle_cmd = resolve_gradle_command(config.project_dir)?;
    let java_home = env::var("DX_ANDROID_JAVA_HOME")
        .or_else(|_| env::var("ANDROID_JAVA_HOME"))
        .or_else(|_| env::var("JAVA_HOME"))
        .ok();
    let sdk_root = env::var("DX_ANDROID_SDK_ROOT")
        .or_else(|_| env::var("ANDROID_SDK_ROOT"))
        .ok();
    let ndk_home = env::var("DX_ANDROID_NDK_HOME")
        .or_else(|_| env::var("ANDROID_NDK_HOME"))
        .ok();

    let mut command = Command::new(&gradle_cmd);
    command
        .arg(config.gradle_task)
        .current_dir(config.project_dir);

    if let Some(ref java_home) = java_home {
        command.env("JAVA_HOME", java_home);
        command.env("DX_ANDROID_JAVA_HOME", java_home);
        let mut gradle_opts = env::var("GRADLE_OPTS").unwrap_or_default();
        if !gradle_opts.is_empty() {
            gradle_opts.push(' ');
        }
        gradle_opts.push_str(&format!("-Dorg.gradle.java.home={java_home}"));
        command.env("GRADLE_OPTS", gradle_opts);
    }
    if let Some(ref sdk_root) = sdk_root {
        command.env("ANDROID_SDK_ROOT", sdk_root);
        command.env("ANDROID_HOME", sdk_root);
        command.env("DX_ANDROID_SDK_ROOT", sdk_root);
    }
    if let Some(ref ndk_home) = ndk_home {
        command.env("ANDROID_NDK_HOME", ndk_home);
        command.env("NDK_HOME", ndk_home);
        command.env("DX_ANDROID_NDK_HOME", ndk_home);
    }

    let status = command.status().map_err(|e| {
        format!(
            "Failed to invoke `{}` while building Android plugin: {e}",
            gradle_cmd
        )
    })?;

    if !status.success() {
        return Err(format!(
            "Gradle build failed while compiling Android plugin using `{gradle_cmd}`"
        )
        .into());
    }

    let mut aar_path = config.preferred_artifact.to_path_buf();
    if !aar_path.exists() {
        aar_path = discover_release_aar(config.project_dir).ok_or_else(|| {
            format!(
                "Expected Android AAR at `{}` or any '*-release.aar' under `{}`",
                config.preferred_artifact.display(),
                config.project_dir.join("build/outputs/aar").display()
            )
        })?;
    }

    let artifact_dir = env::var_os("DX_ANDROID_ARTIFACT_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("OUT_DIR")
                .map(PathBuf::from)
                .map(|dir| dir.join("android-artifacts"))
        })
        .ok_or_else(|| "DX_ANDROID_ARTIFACT_DIR not set and OUT_DIR unavailable".to_string())?;

    fs::create_dir_all(&artifact_dir)?;
    let filename = aar_path
        .file_name()
        .ok_or_else(|| format!("AAR path missing filename: {}", aar_path.display()))?;
    let dest_path = artifact_dir.join(filename);
    fs::copy(&aar_path, &dest_path)?;
    let dest_str = dest_path.to_str().ok_or_else(|| {
        format!(
            "Artifact path contains non-UTF8 characters: {}",
            dest_path.display()
        )
    })?;
    println!("cargo:rustc-env={}={dest_str}", config.artifact_env_key);

    Ok(())
}

fn swift_target_and_sdk(target: &str, min_ios: &str) -> Option<(String, &'static str)> {
    if target.starts_with("aarch64-apple-ios-sim") {
        Some((
            format!("arm64-apple-ios{min_ios}-simulator"),
            "iphonesimulator",
        ))
    } else if target.starts_with("aarch64-apple-ios") {
        Some((format!("arm64-apple-ios{min_ios}"), "iphoneos"))
    } else if target.starts_with("x86_64-apple-ios") {
        Some((
            format!("x86_64-apple-ios{min_ios}-simulator"),
            "iphonesimulator",
        ))
    } else {
        None
    }
}

fn lookup_sdk_path(sdk: &str) -> Result<String> {
    let output = Command::new("xcrun")
        .arg("--sdk")
        .arg(sdk)
        .arg("--show-sdk-path")
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    } else {
        Err(format!(
            "xcrun failed to locate SDK {sdk}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

fn swift_runtime_lib_dir(swift_target: &str) -> Result<PathBuf> {
    let output = Command::new("xcode-select").arg("-p").output()?;
    if !output.status.success() {
        return Err(format!(
            "xcode-select -p failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let developer_dir = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    let toolchain_dir = developer_dir
        .join("Toolchains")
        .join("XcodeDefault.xctoolchain")
        .join("usr")
        .join("lib")
        .join("swift");

    let platform_dir = if swift_target.contains("simulator") {
        "iphonesimulator"
    } else {
        "iphoneos"
    };

    let runtime_dir = toolchain_dir.join(platform_dir);
    if runtime_dir.exists() {
        Ok(runtime_dir)
    } else {
        Err(format!(
            "Swift runtime library directory not found: {}",
            runtime_dir.display()
        )
        .into())
    }
}

fn find_static_lib(
    build_dir: &Path,
    configuration: &str,
    swift_target: &str,
    product: &str,
) -> Option<PathBuf> {
    let lib_name = format!("lib{product}.a");
    let candidates = [
        build_dir
            .join(configuration)
            .join(swift_target)
            .join(&lib_name),
        build_dir
            .join(swift_target)
            .join(configuration)
            .join(&lib_name),
        build_dir.join(configuration).join(&lib_name),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    find_file_recursively(build_dir, &lib_name)
}

fn find_file_recursively(root: &Path, needle: &str) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }

    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() && path.file_name().is_some_and(|n| n == needle) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file_recursively(&path, needle) {
                return Some(found);
            }
        }
    }

    None
}

fn resolve_gradle_command(project_dir: &Path) -> Result<String> {
    if let Ok(cmd) = env::var("GRADLE") {
        return Ok(cmd);
    }

    let gradlew = project_dir.join("gradlew");
    if gradlew.exists() {
        return Ok(gradlew.display().to_string());
    }

    Ok("gradle".to_string())
}

fn discover_release_aar(project_dir: &Path) -> Option<PathBuf> {
    let outputs_dir = project_dir.join("build/outputs/aar");
    if !outputs_dir.exists() {
        return None;
    }

    fs::read_dir(&outputs_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("aar"))
                    .unwrap_or(false)
        })
        .find(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|name| name.ends_with("-release.aar"))
                .unwrap_or(false)
        })
}

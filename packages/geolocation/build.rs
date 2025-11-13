use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const SWIFT_PRODUCT: &str = "GeolocationPlugin";
const SWIFT_MIN_IOS: &str = "13.0";
// Prefer a specific name when present, but fall back to discovering the
// release AAR in the outputs directory to be resilient to AGP naming.
const ANDROID_AAR_PREFERRED: &str = "android/build/outputs/aar/geolocation-plugin-release.aar";

fn main() {
    println!("cargo:rerun-if-changed=ios/Package.swift");
    println!("cargo:rerun-if-changed=ios/Sources/GeolocationPlugin.swift");
    println!("cargo:rerun-if-changed=android/build.gradle.kts");
    println!("cargo:rerun-if-changed=android/settings.gradle.kts");
    println!("cargo:rerun-if-changed=android/src");

    if let Err(err) = build_swift_package() {
        panic!("Failed to build Swift plugin: {err}");
    }

    if let Err(err) = build_android_library() {
        panic!("Failed to build Android plugin: {err}");
    }
}

fn build_swift_package() -> Result<(), Box<dyn Error>> {
    let target = env::var("TARGET")?;
    if !target.contains("apple-ios") {
        return Ok(());
    }

    let (swift_target, sdk_name) = swift_target_and_sdk(&target)
        .ok_or_else(|| format!("Unsupported iOS target `{target}` for Swift compilation"))?;
    let sdk_path = lookup_sdk_path(sdk_name)?;

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let configuration = if profile == "release" {
        "release"
    } else {
        "debug"
    };

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let package_dir = manifest_dir.join("ios");
    let build_dir = PathBuf::from(env::var("OUT_DIR")?).join("swift-build");

    let output = Command::new("xcrun")
        .arg("swift")
        .arg("build")
        .arg("--package-path")
        .arg(&package_dir)
        .arg("--configuration")
        .arg(configuration)
        .arg("--triple")
        .arg(&swift_target)
        .arg("--sdk")
        .arg(&sdk_path)
        .arg("--product")
        .arg(SWIFT_PRODUCT)
        .arg("--build-path")
        .arg(&build_dir)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "swift build failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let lib_path = find_static_lib(&build_dir, configuration, &swift_target, SWIFT_PRODUCT)
        .ok_or_else(|| {
            format!(
                "Could not locate Swift static library for product `{}`",
                SWIFT_PRODUCT
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
    println!("cargo:rustc-link-lib=static={}", SWIFT_PRODUCT);
    println!("cargo:rustc-link-arg=-Xlinker");
    println!("cargo:rustc-link-arg=-force_load");
    println!("cargo:rustc-link-arg=-Xlinker");
    println!("cargo:rustc-link-arg={}", lib_path.display());
    println!("cargo:rustc-link-arg=-ObjC");
    println!("cargo:rustc-link-lib=framework=CoreLocation");
    println!("cargo:rustc-link-lib=framework=Foundation");

    // Swift compatibility shims are required when targeting newer toolchains from lower minimums.
    println!("cargo:rustc-link-lib=swiftCompatibility56");
    println!("cargo:rustc-link-lib=swiftCompatibilityConcurrency");
    println!("cargo:rustc-link-lib=swiftCompatibilityPacks");

    Ok(())
}

fn swift_target_and_sdk(target: &str) -> Option<(String, &'static str)> {
    if target.starts_with("aarch64-apple-ios-sim") {
        Some((
            format!("arm64-apple-ios{SWIFT_MIN_IOS}-simulator"),
            "iphonesimulator",
        ))
    } else if target.starts_with("aarch64-apple-ios") {
        Some((format!("arm64-apple-ios{SWIFT_MIN_IOS}"), "iphoneos"))
    } else if target.starts_with("x86_64-apple-ios") {
        Some((
            format!("x86_64-apple-ios{SWIFT_MIN_IOS}-simulator"),
            "iphonesimulator",
        ))
    } else {
        None
    }
}

fn lookup_sdk_path(sdk: &str) -> Result<String, Box<dyn Error>> {
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

fn swift_runtime_lib_dir(swift_target: &str) -> Result<PathBuf, Box<dyn Error>> {
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

fn resolve_gradle_command(project_dir: &Path) -> Result<String, Box<dyn Error>> {
    if let Ok(cmd) = env::var("GRADLE") {
        return Ok(cmd);
    }

    let gradlew = project_dir.join("gradlew");
    if gradlew.exists() {
        return Ok(gradlew.display().to_string());
    }

    Ok("gradle".to_string())
}

fn build_android_library() -> Result<(), Box<dyn Error>> {
    let target = env::var("TARGET")?;
    if !target.contains("android") {
        return Ok(());
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let project_dir = manifest_dir.join("android");
    let gradle_cmd = resolve_gradle_command(&project_dir)?;
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
    command.arg("assembleRelease").current_dir(&project_dir);

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
            "Failed to invoke `{}` while building Android plugin: {}",
            gradle_cmd, e
        )
    })?;

    if !status.success() {
        return Err(format!(
            "Gradle build failed while compiling Android plugin using `{gradle_cmd}`"
        )
        .into());
    }

    // Locate the built AAR. Prefer the expected fixed name, otherwise
    // discover any `*-release.aar` under the outputs directory.
    let mut aar_path = manifest_dir.join(ANDROID_AAR_PREFERRED);
    if !aar_path.exists() {
        let outputs_dir = manifest_dir.join("android/build/outputs/aar");
        let discovered = fs::read_dir(&outputs_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter(|p| {
                p.extension().is_some_and(|ext| ext == "aar")
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.ends_with("-release.aar"))
            })
            .next();

        if let Some(found) = discovered {
            aar_path = found;
        } else {
            return Err(format!(
                "Expected Android AAR at `{}` or any '*-release.aar' in `{}` but none were found",
                manifest_dir.join(ANDROID_AAR_PREFERRED).display(),
                outputs_dir.display()
            )
            .into());
        }
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
    println!("cargo:rustc-env=DIOXUS_ANDROID_ARTIFACT={dest_str}");

    Ok(())
}

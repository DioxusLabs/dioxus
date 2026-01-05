//! iOS/macOS Swift package manifest helpers and compilation.

use crate::Result;
use anyhow::Context;
use manganis_core::SwiftPackageMetadata;
use std::path::{Path, PathBuf};
use target_lexicon::{OperatingSystem, Triple};
use tokio::process::Command;

/// Create a proper framework bundle from a dylib for iOS/macOS.
///
/// iOS uses a flat structure while macOS uses a versioned structure.
/// Both require an Info.plist for proper App Store submission.
pub async fn create_framework_bundle(
    dylib_path: &Path,
    framework_name: &str,
    output_dir: &Path,
    target_triple: &Triple,
    bundle_identifier: &str,
) -> Result<PathBuf> {
    let is_ios = matches!(target_triple.operating_system, OperatingSystem::IOS(_));
    let min_os_version = if is_ios { "13.0" } else { "11.0" };

    let framework_dir = output_dir.join(format!("{}.framework", framework_name));

    // Remove existing framework if present
    if framework_dir.exists() {
        std::fs::remove_dir_all(&framework_dir)?;
    }

    if is_ios {
        // iOS uses flat structure: Framework.framework/FrameworkName + Info.plist
        std::fs::create_dir_all(&framework_dir)?;

        // Copy dylib as the framework executable (no extension)
        let exec_path = framework_dir.join(framework_name);
        std::fs::copy(dylib_path, &exec_path)?;

        // Set the install name using install_name_tool
        let output = Command::new("xcrun")
            .arg("install_name_tool")
            .arg("-id")
            .arg(format!(
                "@rpath/{}.framework/{}",
                framework_name, framework_name
            ))
            .arg(&exec_path)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("install_name_tool failed: {}", stderr);
        }

        // Create Info.plist
        let info_plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>{framework_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_identifier}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{framework_name}</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>MinimumOSVersion</key>
    <string>{min_os_version}</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>iPhoneOS</string>
    </array>
</dict>
</plist>"#
        );

        std::fs::write(framework_dir.join("Info.plist"), info_plist)?;
    } else {
        // macOS uses versioned structure with symlinks
        let versions_a = framework_dir.join("Versions").join("A");
        let resources_dir = versions_a.join("Resources");
        std::fs::create_dir_all(&resources_dir)?;

        // Copy dylib as the framework executable
        let exec_path = versions_a.join(framework_name);
        std::fs::copy(dylib_path, &exec_path)?;

        // Set install name
        let output = Command::new("xcrun")
            .arg("install_name_tool")
            .arg("-id")
            .arg(format!(
                "@rpath/{}.framework/Versions/A/{}",
                framework_name, framework_name
            ))
            .arg(&exec_path)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("install_name_tool failed: {}", stderr);
        }

        // Create Info.plist in Resources
        let info_plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>{framework_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_identifier}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{framework_name}</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>{min_os_version}</string>
</dict>
</plist>"#
        );

        std::fs::write(resources_dir.join("Info.plist"), info_plist)?;

        // Create symbolic links (required for macOS framework structure)
        let versions_dir = framework_dir.join("Versions");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("A", versions_dir.join("Current"))?;
            std::os::unix::fs::symlink(
                format!("Versions/Current/{}", framework_name),
                framework_dir.join(framework_name),
            )?;
            std::os::unix::fs::symlink(
                "Versions/Current/Resources",
                framework_dir.join("Resources"),
            )?;
        }
    }

    tracing::info!(
        "Created {} framework bundle: {}",
        if is_ios { "iOS" } else { "macOS" },
        framework_dir.display()
    );

    Ok(framework_dir)
}

/// Compile Swift sources and return the path to the dynamic framework bundle.
///
/// This function:
/// 1. Generates an umbrella Package.swift that includes all Swift plugins
/// 2. Runs `swift build` to compile into a dynamic library
/// 3. Wraps the dylib in a proper .framework bundle for iOS/macOS
/// 4. Returns the path to the resulting `.framework` bundle
pub async fn compile_swift_sources(
    swift_sources: &[SwiftPackageMetadata],
    target_triple: &Triple,
    build_dir: &Path,
    release: bool,
) -> Result<Option<PathBuf>> {
    if swift_sources.is_empty() {
        return Ok(None);
    }

    tracing::info!(
        "Compiling {} Swift plugin(s) for {}",
        swift_sources.len(),
        target_triple
    );

    // Create the plugins build directory
    let plugins_dir = build_dir.join("swift-plugins");
    std::fs::create_dir_all(&plugins_dir)?;

    // Copy and prepare all Swift source packages
    let mut plugin_paths = Vec::new();
    for source in swift_sources {
        let source_path = PathBuf::from(source.package_path.as_str());
        let plugin_name = source.plugin_name.as_str();
        let product_name = source.product.as_str();

        if !source_path.exists() {
            tracing::warn!(
                "Swift package path does not exist: {} (for plugin {})",
                source_path.display(),
                plugin_name
            );
            continue;
        }

        let dest_path = plugins_dir.join(plugin_name);
        if dest_path.exists() {
            std::fs::remove_dir_all(&dest_path)?;
        }
        copy_dir_recursive(&source_path, &dest_path)?;

        // Modify the Package.swift to produce a dynamic library
        if let Err(e) = modify_package_for_dynamic_library(&dest_path, product_name) {
            tracing::warn!(
                "Failed to modify Package.swift for dynamic library: {}",
                e
            );
        }

        plugin_paths.push((plugin_name.to_string(), product_name.to_string(), dest_path));
        tracing::debug!(
            "Copied Swift plugin '{}' from {} to {}",
            plugin_name,
            source_path.display(),
            plugins_dir.join(plugin_name).display()
        );
    }

    if plugin_paths.is_empty() {
        tracing::warn!("No valid Swift packages found to compile");
        return Ok(None);
    }

    // Determine Swift target triple and SDK
    let (swift_triple, sdk_name) = swift_target_and_sdk(target_triple)?;
    let sdk_path = lookup_sdk_path(&sdk_name).await?;

    // Build configuration
    let configuration = if release { "release" } else { "debug" };

    // Build each plugin package individually
    for (plugin_name, product_name, package_path) in &plugin_paths {
        tracing::info!(
            "Building Swift plugin '{}' (product: {})",
            plugin_name,
            product_name
        );

        let build_path = package_path.join(".build");

        let mut cmd = Command::new("xcrun");
        cmd.args(["swift", "build"])
            .arg("--package-path")
            .arg(package_path)
            .arg("--configuration")
            .arg(configuration)
            .arg("--triple")
            .arg(&swift_triple)
            .arg("--sdk")
            .arg(&sdk_path)
            .arg("--product")
            .arg(product_name)
            .arg("--build-path")
            .arg(&build_path);

        tracing::debug!("Running: xcrun swift build for {}", product_name);

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "Swift build failed for plugin '{}':\n{}\n{}",
                plugin_name,
                stdout,
                stderr
            );
        }

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::debug!("Swift build warnings for {}:\n{}", plugin_name, stderr);
        }
    }

    // Find the output dynamic library for each plugin
    // Swift puts the output in .build/<triple>/<configuration>/lib<ProductName>.dylib
    // or .build/<configuration>/lib<ProductName>.dylib depending on the version
    let mut all_dylibs = Vec::new();

    for (_, product_name, package_path) in &plugin_paths {
        let build_path = package_path.join(".build");
        let lib_name = format!("lib{}.dylib", product_name);

        let lib_search_paths = [
            build_path.join(&swift_triple).join(configuration),
            build_path.join(configuration),
            build_path.clone(),
        ];

        let mut found = false;
        for search_path in &lib_search_paths {
            let lib_path = search_path.join(&lib_name);
            if lib_path.exists() {
                tracing::debug!("Found Swift dynamic library: {}", lib_path.display());
                all_dylibs.push((product_name.clone(), lib_path));
                found = true;
                break;
            }
        }

        if !found {
            tracing::warn!(
                "Could not find compiled Swift dynamic library for product '{}' (expected {})",
                product_name,
                lib_name
            );
        }
    }

    if all_dylibs.is_empty() {
        tracing::warn!("No Swift dynamic libraries were compiled successfully");
        return Ok(None);
    }

    // For dynamic libraries, we need to wrap each in a framework bundle
    // If there's only one library, create a single framework
    // If there are multiple, we'll create frameworks for each (they're independent)
    // The first one is the "primary" framework that gets returned

    let (_primary_name, primary_dylib) = all_dylibs.remove(0);

    // Create the framework bundle from the dylib
    // Use "DioxusSwiftPlugins" as the umbrella framework name
    let framework_name = "DioxusSwiftPlugins";
    let bundle_identifier = "com.dioxus.swift.plugins";

    let framework_path = create_framework_bundle(
        &primary_dylib,
        framework_name,
        build_dir,
        target_triple,
        bundle_identifier,
    )
    .await?;

    // If there are additional dylibs, create separate framework bundles for them
    for (name, dylib_path) in all_dylibs {
        let extra_framework = create_framework_bundle(
            &dylib_path,
            &name,
            build_dir,
            target_triple,
            &format!("com.dioxus.swift.{}", name.to_lowercase()),
        )
        .await?;
        tracing::info!("Created additional framework: {}", extra_framework.display());
    }

    Ok(Some(framework_path))
}

/// Modify a Package.swift to produce a dynamic library instead of static.
/// This is needed for runtime class lookup via NSClassFromString.
fn modify_package_for_dynamic_library(package_path: &Path, product_name: &str) -> Result<()> {
    let package_swift_path = package_path.join("Package.swift");
    if !package_swift_path.exists() {
        anyhow::bail!(
            "Package.swift not found at {}",
            package_swift_path.display()
        );
    }

    let content = std::fs::read_to_string(&package_swift_path)?;

    // Replace .static with .dynamic for the library type
    let modified = content
        .replace("type: .static", "type: .dynamic")
        .replace("type:.static", "type: .dynamic");

    // If no library type was specified, we need to add it
    // Look for .library(name: "ProductName", targets: [...]) and change to
    // .library(name: "ProductName", type: .dynamic, targets: [...])
    let pattern = format!(
        r#".library\s*\(\s*name\s*:\s*"{}"\s*,\s*targets"#,
        regex::escape(product_name)
    );
    let replacement = format!(r#".library(name: "{}", type: .dynamic, targets"#, product_name);

    let modified = if let Ok(re) = regex::Regex::new(&pattern) {
        re.replace_all(&modified, replacement.as_str()).to_string()
    } else {
        modified
    };

    std::fs::write(&package_swift_path, modified)?;
    Ok(())
}

/// Convert a Rust target triple to Swift target triple and SDK name
fn swift_target_and_sdk(triple: &Triple) -> Result<(String, String)> {
    use target_lexicon::{Architecture, Environment, OperatingSystem};

    // Check if this is a simulator target using the environment field
    let is_simulator = triple.environment == Environment::Sim;

    let swift_triple = match (&triple.architecture, &triple.operating_system) {
        (Architecture::Aarch64(_), OperatingSystem::IOS(_)) => {
            if is_simulator {
                "arm64-apple-ios-simulator"
            } else {
                "arm64-apple-ios"
            }
        }
        (Architecture::Aarch64(_), OperatingSystem::MacOSX { .. } | OperatingSystem::Darwin(_)) => {
            "arm64-apple-macosx"
        }
        (Architecture::X86_64, OperatingSystem::IOS(_)) => "x86_64-apple-ios-simulator",
        (Architecture::X86_64, OperatingSystem::MacOSX { .. } | OperatingSystem::Darwin(_)) => {
            "x86_64-apple-macosx"
        }
        _ => anyhow::bail!("Unsupported target for Swift compilation: {}", triple),
    };

    let sdk_name = match &triple.operating_system {
        OperatingSystem::IOS(_) => {
            // Check if this is a simulator target using the environment field
            if is_simulator {
                "iphonesimulator"
            } else {
                "iphoneos"
            }
        }
        OperatingSystem::MacOSX { .. } | OperatingSystem::Darwin(_) => "macosx",
        _ => anyhow::bail!(
            "Unsupported operating system for Swift compilation: {:?}",
            triple.operating_system
        ),
    };

    Ok((swift_triple.to_string(), sdk_name.to_string()))
}

/// Look up the SDK path using xcrun
async fn lookup_sdk_path(sdk_name: &str) -> Result<String> {
    let output = Command::new("xcrun")
        .args(["--sdk", sdk_name, "--show-sdk-path"])
        .output()
        .await
        .context("Failed to run xcrun to find SDK path")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to find SDK '{}': {}", sdk_name, stderr);
    }

    let sdk_path = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in SDK path")?
        .trim()
        .to_string();

    if sdk_path.is_empty() {
        anyhow::bail!("SDK path for '{}' is empty", sdk_name);
    }

    Ok(sdk_path)
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            // Skip .build directories
            if entry.file_name() == ".build" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Extract Swift metadata from object files in link arguments
pub fn extract_swift_metadata_from_link_args(
    link_args: &[String],
    workspace_dir: &Path,
) -> Vec<SwiftPackageMetadata> {
    let mut swift_packages = Vec::new();

    // Look through rlibs and object files for Swift metadata
    for arg in link_args {
        let path = PathBuf::from(arg);

        // Only process files in our workspace
        if !path.starts_with(workspace_dir) {
            continue;
        }

        // Check for .rlib files
        if arg.ends_with(".rlib") {
            if let Ok(swift_meta) = extract_swift_from_rlib(&path) {
                swift_packages.extend(swift_meta);
            }
        }
        // Check for .o files
        else if arg.ends_with(".o") || arg.ends_with(".obj") {
            if let Ok(swift_meta) = extract_swift_from_object(&path) {
                swift_packages.extend(swift_meta);
            }
        }
    }

    // Deduplicate by plugin name
    swift_packages.sort_by(|a, b| a.plugin_name.as_str().cmp(b.plugin_name.as_str()));
    swift_packages.dedup_by(|a, b| a.plugin_name.as_str() == b.plugin_name.as_str());

    swift_packages
}

/// Extract Swift metadata from an rlib file
fn extract_swift_from_rlib(rlib_path: &Path) -> Result<Vec<SwiftPackageMetadata>> {
    let mut results = Vec::new();

    let rlib_contents = std::fs::read(rlib_path)?;
    let mut reader = ar::Archive::new(std::io::Cursor::new(rlib_contents));

    while let Some(Ok(entry)) = reader.next_entry() {
        let name = std::str::from_utf8(entry.header().identifier()).unwrap_or_default();

        // Only process .o files
        if !name.ends_with(".rcgu.o") && !name.ends_with(".obj") {
            continue;
        }

        // Read the object file contents
        let mut obj_contents = Vec::new();
        std::io::Read::read_to_end(&mut std::io::BufReader::new(entry), &mut obj_contents)?;

        if let Ok(swift_meta) = extract_swift_from_bytes(&obj_contents) {
            results.extend(swift_meta);
        }
    }

    Ok(results)
}

/// Extract Swift metadata from an object file
fn extract_swift_from_object(obj_path: &Path) -> Result<Vec<SwiftPackageMetadata>> {
    let obj_contents = std::fs::read(obj_path)?;
    extract_swift_from_bytes(&obj_contents)
}

/// Extract Swift metadata from raw object file bytes
fn extract_swift_from_bytes(bytes: &[u8]) -> Result<Vec<SwiftPackageMetadata>> {
    use manganis_core::SymbolData;
    use object::{Object, ObjectSection, ObjectSymbol};

    let mut results = Vec::new();

    let file = match object::File::parse(bytes) {
        Ok(f) => f,
        Err(_) => return Ok(results),
    };

    // Look for __ASSETS__ symbols
    for symbol in file.symbols() {
        let name = match symbol.name() {
            Ok(n) => n,
            Err(_) => continue,
        };

        if !name.starts_with("__ASSETS__") {
            continue;
        }

        // Try to get the symbol's data
        if let Some(section_idx) = symbol.section().index() {
            if let Ok(section) = file.section_by_index(section_idx) {
                if let Ok(data) = section.data() {
                    // Try to find the symbol data in the section
                    let addr = symbol.address();
                    let section_addr = section.address();
                    let offset = (addr - section_addr) as usize;

                    if offset < data.len() {
                        let symbol_data = &data[offset..];
                        // Try to deserialize as SymbolData
                        if let Some((_, symbol_data)) =
                            const_serialize::deserialize_const!(SymbolData, symbol_data)
                        {
                            // swift pm is no longer stored as a metadata symbol.
                            if let SymbolData::SwiftPackage(meta) = symbol_data {
                                results.push(meta);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

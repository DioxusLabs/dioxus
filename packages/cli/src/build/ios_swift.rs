//! iOS/macOS Swift package manifest helpers and compilation.

use crate::Result;
use anyhow::Context;
use const_serialize::{ConstStr, SerializeConst};
use std::path::{Path, PathBuf};
use target_lexicon::Triple;
use tokio::process::Command;
use SwiftPackageMetadata as SwiftSourceMetadata;

/// Manifest of Swift packages embedded in the binary.
#[derive(Debug, Clone, Default)]
pub struct SwiftSourceManifest {
    sources: Vec<SwiftSourceMetadata>,
}

impl SwiftSourceManifest {
    pub fn new(sources: Vec<SwiftSourceMetadata>) -> Self {
        Self { sources }
    }

    pub fn sources(&self) -> &[SwiftSourceMetadata] {
        &self.sources
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

/// Compile Swift sources and return the path to the static library.
///
/// This function:
/// 1. Generates an umbrella Package.swift that includes all Swift plugins
/// 2. Runs `swift build` to compile into a static library
/// 3. Returns the path to the resulting `.a` file
pub async fn compile_swift_sources(
    swift_sources: &[SwiftSourceMetadata],
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

    // Create the umbrella package directory
    let umbrella_dir = build_dir.join("swift-plugins");
    std::fs::create_dir_all(&umbrella_dir)?;

    // Copy all Swift source packages to the umbrella directory
    let mut local_packages = Vec::new();
    for source in swift_sources {
        let source_path = PathBuf::from(source.package_path.as_str());
        let plugin_name = source.plugin_name.as_str();

        if !source_path.exists() {
            tracing::warn!(
                "Swift package path does not exist: {} (for plugin {})",
                source_path.display(),
                plugin_name
            );
            continue;
        }

        let dest_path = umbrella_dir.join(plugin_name);
        if dest_path.exists() {
            std::fs::remove_dir_all(&dest_path)?;
        }
        copy_dir_recursive(&source_path, &dest_path)?;

        local_packages.push((plugin_name.to_string(), source.product.as_str().to_string()));
        tracing::debug!(
            "Copied Swift plugin '{}' from {} to {}",
            plugin_name,
            source_path.display(),
            dest_path.display()
        );
    }

    if local_packages.is_empty() {
        tracing::warn!("No valid Swift packages found to compile");
        return Ok(None);
    }

    // Generate the umbrella Package.swift
    let package_swift = generate_umbrella_package_swift(&local_packages);
    let package_swift_path = umbrella_dir.join("Package.swift");
    std::fs::write(&package_swift_path, package_swift)?;
    tracing::debug!(
        "Generated umbrella Package.swift at {}",
        package_swift_path.display()
    );

    // Determine Swift target triple and SDK
    let (swift_triple, sdk_name) = swift_target_and_sdk(target_triple)?;
    let sdk_path = lookup_sdk_path(&sdk_name).await?;

    // Build configuration
    let configuration = if release { "release" } else { "debug" };

    // Build all products
    let build_path = umbrella_dir.join(".build");

    for (plugin_name, product_name) in &local_packages {
        tracing::info!(
            "Building Swift plugin '{}' (product: {})",
            plugin_name,
            product_name
        );

        let mut cmd = Command::new("xcrun");
        cmd.args(["swift", "build"])
            .arg("--package-path")
            .arg(&umbrella_dir)
            .arg("--configuration")
            .arg(configuration)
            .arg("--triple")
            .arg(&swift_triple)
            .arg("--sdk")
            .arg(&sdk_path)
            .arg("--product")
            .arg(product_name)
            .arg("--build-path")
            .arg(&build_path)
            // Build as static library
            .arg("-Xswiftc")
            .arg("-static");

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

    // Find the output static library
    // Swift puts the output in .build/<triple>/<configuration>/lib<ProductName>.a
    // or .build/<configuration>/lib<ProductName>.a depending on the version
    let lib_search_paths = [
        build_path.join(&swift_triple).join(configuration),
        build_path.join(configuration),
    ];

    // Create a merged static library from all plugins
    let merged_lib_path = build_dir.join("libswift_plugins.a");
    let mut all_libs = Vec::new();

    for (_, product_name) in &local_packages {
        let lib_name = format!("lib{}.a", product_name);
        let mut found = false;

        for search_path in &lib_search_paths {
            let lib_path = search_path.join(&lib_name);
            if lib_path.exists() {
                tracing::debug!("Found Swift library: {}", lib_path.display());
                all_libs.push(lib_path);
                found = true;
                break;
            }
        }

        if !found {
            // Also check for the static library directly
            let lib_path = build_path.join(&lib_name);
            if lib_path.exists() {
                all_libs.push(lib_path);
            } else {
                tracing::warn!(
                    "Could not find compiled Swift library for product '{}' (expected {})",
                    product_name,
                    lib_name
                );
            }
        }
    }

    if all_libs.is_empty() {
        tracing::warn!("No Swift libraries were compiled successfully");
        return Ok(None);
    }

    // If there's only one library, just return it
    if all_libs.len() == 1 {
        return Ok(Some(all_libs.remove(0)));
    }

    // Otherwise, merge all libraries into one using libtool
    tracing::debug!(
        "Merging {} Swift libraries into {}",
        all_libs.len(),
        merged_lib_path.display()
    );

    let mut cmd = Command::new("xcrun");
    cmd.arg("libtool")
        .arg("-static")
        .arg("-o")
        .arg(&merged_lib_path);

    for lib in &all_libs {
        cmd.arg(lib);
    }

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to merge Swift libraries: {}", stderr);
    }

    Ok(Some(merged_lib_path))
}

/// Generate an umbrella Package.swift that includes all local Swift packages
fn generate_umbrella_package_swift(packages: &[(String, String)]) -> String {
    let mut swift = String::from(
        r#"// swift-tools-version:5.7
// Auto-generated umbrella package for Dioxus Swift plugins

import PackageDescription

let package = Package(
    name: "DioxusSwiftPlugins",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15)
    ],
    products: [
"#,
    );

    // Add products
    for (_, product_name) in packages {
        swift.push_str(&format!(
            "        .library(name: \"{}\", type: .static, targets: [\"{}\"]),\n",
            product_name, product_name
        ));
    }

    swift.push_str(
        r#"    ],
    dependencies: [
"#,
    );

    // Add local package dependencies
    for (plugin_name, _) in packages {
        swift.push_str(&format!("        .package(path: \"./{}\")\n", plugin_name));
    }

    swift.push_str(
        r#"    ],
    targets: [
"#,
    );

    // Add targets that depend on the local packages
    for (plugin_name, product_name) in packages {
        swift.push_str(&format!(
            "        .target(name: \"{}\", dependencies: [.product(name: \"{}\", package: \"{}\")]),\n",
            product_name, product_name, plugin_name
        ));
    }

    swift.push_str(
        r#"    ]
)
"#,
    );

    swift
}

/// Convert a Rust target triple to Swift target triple and SDK name
fn swift_target_and_sdk(triple: &Triple) -> Result<(String, String)> {
    use target_lexicon::{Architecture, OperatingSystem};

    let swift_triple = match (&triple.architecture, &triple.operating_system) {
        (Architecture::Aarch64(_), OperatingSystem::IOS(_)) => "arm64-apple-ios",
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
            // Check if this is a simulator target
            if triple.architecture == Architecture::X86_64 {
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
) -> Vec<SwiftSourceMetadata> {
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
fn extract_swift_from_rlib(rlib_path: &Path) -> Result<Vec<SwiftSourceMetadata>> {
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
fn extract_swift_from_object(obj_path: &Path) -> Result<Vec<SwiftSourceMetadata>> {
    let obj_contents = std::fs::read(obj_path)?;
    extract_swift_from_bytes(&obj_contents)
}

/// Extract Swift metadata from raw object file bytes
fn extract_swift_from_bytes(bytes: &[u8]) -> Result<Vec<SwiftSourceMetadata>> {
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
                            // if let SymbolData::SwiftPackage(meta) = symbol_data {
                            //     results.push(meta);
                            // }
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Metadata describing an Android plugin artifact (.aar) that must be copied into the host Gradle project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeConst)]
pub struct AndroidArtifactMetadata {
    pub plugin_name: ConstStr,
    pub artifact_path: ConstStr,
    pub gradle_dependencies: ConstStr,
}

impl AndroidArtifactMetadata {
    pub const fn new(
        plugin_name: &'static str,
        artifact_path: &'static str,
        gradle_dependencies: &'static str,
    ) -> Self {
        Self {
            plugin_name: ConstStr::new(plugin_name),
            artifact_path: ConstStr::new(artifact_path),
            gradle_dependencies: ConstStr::new(gradle_dependencies),
        }
    }
}

/// Metadata for a Swift package that needs to be linked into the app (iOS/macOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeConst)]
pub struct SwiftPackageMetadata {
    pub plugin_name: ConstStr,
    pub package_path: ConstStr,
    pub product: ConstStr,
}

impl SwiftPackageMetadata {
    pub const fn new(
        plugin_name: &'static str,
        package_path: &'static str,
        product: &'static str,
    ) -> Self {
        Self {
            plugin_name: ConstStr::new(plugin_name),
            package_path: ConstStr::new(package_path),
            product: ConstStr::new(product),
        }
    }
}

/// Manifest of all Android artifacts declared by dependencies.
#[derive(Debug, Clone, Default)]
pub struct AndroidArtifactManifest {
    artifacts: Vec<AndroidArtifactMetadata>,
}

impl AndroidArtifactManifest {
    pub fn new(artifacts: Vec<AndroidArtifactMetadata>) -> Self {
        Self { artifacts }
    }

    pub fn artifacts(&self) -> &[AndroidArtifactMetadata] {
        &self.artifacts
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

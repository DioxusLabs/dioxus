use super::tools::{self, ResolvedTools};
use crate::{BuildRequest, DebianSettings, MacOsSettings, PackageType, WindowsSettings};
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// The architecture of the target binary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Arch {
    X86_64,
    X86,
    AArch64,
    Armhf,
    Armel,
    Riscv64,
    Universal,
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::X86 => write!(f, "x86"),
            Arch::AArch64 => write!(f, "aarch64"),
            Arch::Armhf => write!(f, "armhf"),
            Arch::Armel => write!(f, "armel"),
            Arch::Riscv64 => write!(f, "riscv64"),
            Arch::Universal => write!(f, "universal"),
        }
    }
}

/// BundleContext wraps a BuildRequest and provides the settings API
/// that bundle format modules need. This is the adapter between
/// Dioxus build infrastructure and the individual bundler implementations.
pub(crate) struct BundleContext<'a> {
    pub(crate) build: &'a BuildRequest,
    pub(crate) package_types: Vec<PackageType>,
    /// Pre-computed resource map: source path -> target path in bundle
    pub(crate) resources_map: HashMap<String, String>,
    /// Pre-resolved tool paths (NSIS, WiX, linuxdeploy, WebView2).
    pub(crate) tools: ResolvedTools,
}

impl<'a> BundleContext<'a> {
    /// Create a new BundleContext from a BuildRequest and optional package types.
    pub(crate) fn new(
        build: &'a BuildRequest,
        package_types: &Option<Vec<PackageType>>,
    ) -> Result<Self> {
        let package_types = package_types.clone().unwrap_or_default();

        // Build the resources map from assets + config resources
        let mut resources_map = HashMap::new();

        let asset_dir = build.asset_dir();
        if asset_dir.exists() {
            for entry in walkdir::WalkDir::new(&asset_dir) {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let old = path
                        .canonicalize()
                        .with_context(|| format!("Failed to canonicalize {entry:?}"))?;
                    let new = PathBuf::from("assets")
                        .join(path.strip_prefix(&asset_dir).unwrap_or(path));
                    resources_map.insert(old.display().to_string(), new.display().to_string());
                }
            }
        }

        // Merge in any custom resources from config
        if let Some(resources) = &build.config.bundle.resources {
            for resource_path in resources {
                resources_map.insert(resource_path.clone(), String::new());
            }
        }

        let tools_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("dioxus");
        let _ = std::fs::create_dir_all(&tools_dir);

        let arch = {
            let target = build.triple.to_string();
            if target.starts_with("x86_64") {
                Arch::X86_64
            } else if target.starts_with('i') {
                Arch::X86
            } else if target.starts_with("arm") && target.ends_with("hf") {
                Arch::Armhf
            } else if target.starts_with("arm") {
                Arch::Armel
            } else if target.starts_with("aarch64") {
                Arch::AArch64
            } else if target.starts_with("riscv64") {
                Arch::Riscv64
            } else if target.starts_with("universal") {
                Arch::Universal
            } else if cfg!(target_arch = "x86_64") {
                Arch::X86_64
            } else if cfg!(target_arch = "aarch64") {
                Arch::AArch64
            } else {
                Arch::X86_64
            }
        };

        let windows_settings = build.config.bundle.windows.clone().unwrap_or_default();
        let tools = tools::resolve_tools(&tools_dir, &package_types, &windows_settings, arch)?;

        Ok(Self {
            build,
            package_types,
            resources_map,
            tools,
        })
    }

    /// The package types to bundle.
    pub(crate) fn package_types(&self) -> Vec<PackageType> {
        self.package_types.clone()
    }

    /// The product name (PascalCase).
    pub(crate) fn product_name(&self) -> String {
        self.build.bundled_app_name()
    }

    /// The bundle identifier (e.g. "com.example.app").
    pub(crate) fn bundle_identifier(&self) -> String {
        self.build.bundle_identifier()
    }

    /// The publisher name.
    pub(crate) fn publisher(&self) -> Option<&str> {
        self.build.config.bundle.publisher.as_deref()
    }

    /// The version string from Cargo.toml.
    pub(crate) fn version_string(&self) -> String {
        self.build.package().version.to_string()
    }

    /// The short description.
    pub(crate) fn short_description(&self) -> String {
        self.build
            .config
            .bundle
            .short_description
            .clone()
            .or_else(|| self.build.package().description.clone())
            .unwrap_or_default()
    }

    /// The long description.
    pub(crate) fn long_description(&self) -> Option<&str> {
        self.build.config.bundle.long_description.as_deref()
    }

    /// The copyright string.
    pub(crate) fn copyright_string(&self) -> Option<&str> {
        self.build.config.bundle.copyright.as_deref()
    }

    /// The authors list.
    pub(crate) fn authors(&self) -> &[String] {
        &self.build.package().authors
    }

    /// Authors as a comma-separated string.
    pub(crate) fn authors_comma_separated(&self) -> Option<String> {
        let names = self.authors();
        if names.is_empty() {
            None
        } else {
            Some(names.join(", "))
        }
    }

    /// The homepage URL.
    pub(crate) fn homepage_url(&self) -> Option<String> {
        self.build
            .package()
            .homepage
            .as_ref()
            .filter(|hp| !hp.is_empty())
            .cloned()
    }

    /// The license string.
    pub(crate) fn license(&self) -> Option<&str> {
        self.build.package().license.as_deref()
    }

    /// The app category string.
    pub(crate) fn app_category(&self) -> Option<&str> {
        self.build.config.bundle.category.as_deref()
    }

    /// The main binary name (without extension).
    pub(crate) fn main_binary_name(&self) -> &str {
        self.build.executable_name()
    }

    /// The path to the main binary.
    pub(crate) fn main_binary_path(&self) -> PathBuf {
        self.build.main_exe()
    }

    /// The output directory for bundles.
    pub(crate) fn project_out_directory(&self) -> PathBuf {
        self.build.bundle_dir(self.build.bundle)
    }

    /// The target triple string.
    pub(crate) fn target(&self) -> String {
        self.build.triple.to_string()
    }

    /// The binary architecture.
    pub(crate) fn binary_arch(&self) -> Arch {
        let target = self.target();
        if target.starts_with("x86_64") {
            Arch::X86_64
        } else if target.starts_with('i') {
            Arch::X86
        } else if target.starts_with("arm") && target.ends_with("hf") {
            Arch::Armhf
        } else if target.starts_with("arm") {
            Arch::Armel
        } else if target.starts_with("aarch64") {
            Arch::AArch64
        } else if target.starts_with("riscv64") {
            Arch::Riscv64
        } else if target.starts_with("universal") {
            Arch::Universal
        } else {
            // Default to the host arch
            if cfg!(target_arch = "x86_64") {
                Arch::X86_64
            } else if cfg!(target_arch = "aarch64") {
                Arch::AArch64
            } else {
                Arch::X86_64
            }
        }
    }

    /// Icon file paths from config, canonicalized relative to the crate dir.
    pub(crate) fn icon_files(&self) -> Result<Vec<PathBuf>> {
        let mut icons = Vec::new();
        if let Some(icon_paths) = &self.build.config.bundle.icon {
            for icon in icon_paths {
                let icon_path = self
                    .build
                    .crate_dir()
                    .join(icon)
                    .canonicalize()
                    .with_context(|| format!("Failed to canonicalize icon path: {icon}"))?;
                icons.push(icon_path);
            }
        }
        Ok(icons)
    }

    /// Copy resources to the given path.
    pub(crate) fn copy_resources(&self, dest: &Path) -> Result<()> {
        for (src, target) in &self.resources_map {
            let src_path = PathBuf::from(src);
            if !src_path.exists() {
                tracing::warn!("Resource not found: {src}");
                continue;
            }
            let dest_path = if target.is_empty() {
                dest.join(src_path.file_name().unwrap_or_default())
            } else {
                dest.join(target)
            };
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src_path, &dest_path)
                .with_context(|| format!("Failed to copy resource {src} -> {}", dest_path.display()))?;
        }
        Ok(())
    }

    /// Copy external binaries to the given path.
    pub(crate) fn copy_external_binaries(&self, dest: &Path) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        if let Some(bins) = &self.build.config.bundle.external_bin {
            let target = self.target();
            for bin in bins {
                let src = PathBuf::from(format!("{bin}-{target}"));
                if src.exists() {
                    let dest_name = src
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .replace(&format!("-{target}"), "");
                    let dest_path = dest.join(dest_name);
                    std::fs::copy(&src, &dest_path)?;
                    paths.push(dest_path);
                }
            }
        }
        Ok(paths)
    }

    /// The crate directory (where Cargo.toml is).
    pub(crate) fn crate_dir(&self) -> PathBuf {
        self.build.crate_dir()
    }

    /// Debian settings from config.
    pub(crate) fn deb(&self) -> DebianSettings {
        self.build.config.bundle.deb.clone().unwrap_or_default()
    }

    /// macOS settings from config.
    pub(crate) fn macos(&self) -> MacOsSettings {
        self.build.config.bundle.macos.clone().unwrap_or_default()
    }

    /// Windows settings from config.
    pub(crate) fn windows(&self) -> WindowsSettings {
        self.build.config.bundle.windows.clone().unwrap_or_default()
    }

}

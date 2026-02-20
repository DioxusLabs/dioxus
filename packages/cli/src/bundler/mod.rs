mod category;
mod context;
#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
mod tools;
mod updater;
pub(crate) mod windows;

pub(crate) use context::BundleContext;

use crate::PackageType;
use anyhow::Result;
use std::path::PathBuf;

/// A completed bundle with its output paths.
#[derive(Debug)]
pub(crate) struct Bundle {
    pub package_type: PackageType,
    pub bundle_paths: Vec<PathBuf>,
}

/// Bundles the project for the given package types.
/// Returns the list of bundles with their output paths.
pub(crate) fn bundle_project(ctx: &BundleContext) -> Result<Vec<Bundle>> {
    let mut package_types = ctx.package_types();
    if package_types.is_empty() {
        return Ok(Vec::new());
    }

    // Sort so dependencies come first (e.g. .app before .dmg)
    package_types.sort_by_key(|a| a.priority());

    let mut bundles = Vec::<Bundle>::new();

    for package_type in &package_types {
        // Skip if already built (e.g. DMG already built .app)
        if bundles.iter().any(|b| b.package_type == *package_type) {
            continue;
        }

        let bundle_paths = match package_type {
            #[cfg(target_os = "macos")]
            PackageType::MacOsBundle => macos::app::bundle_project(ctx)?,
            #[cfg(target_os = "macos")]
            PackageType::Dmg => {
                let bundled = macos::dmg::bundle_project(ctx, &bundles)?;
                if !bundled.app.is_empty() {
                    bundles.push(Bundle {
                        package_type: PackageType::MacOsBundle,
                        bundle_paths: bundled.app,
                    });
                }
                bundled.dmg
            }
            #[cfg(target_os = "linux")]
            PackageType::Deb => linux::debian::bundle_project(ctx)?,
            #[cfg(target_os = "linux")]
            PackageType::Rpm => linux::rpm::bundle_project(ctx)?,
            #[cfg(target_os = "linux")]
            PackageType::AppImage => linux::appimage::bundle_project(ctx)?,
            #[cfg(target_os = "windows")]
            PackageType::WindowsMsi => windows::msi::bundle_project(ctx)?,
            PackageType::Nsis => windows::nsis::bundle_project(ctx)?,
            PackageType::Updater => updater::bundle_project(ctx, &bundles)?,
            _ => {
                tracing::warn!("Ignoring unsupported package type: {:?}", package_type);
                continue;
            }
        };

        bundles.push(Bundle {
            package_type: *package_type,
            bundle_paths,
        });
    }

    // On macOS, clean up .app if only building dmg or updater
    #[cfg(target_os = "macos")]
    if !package_types.contains(&PackageType::MacOsBundle) {
        if let Some(idx) = bundles
            .iter()
            .position(|b| b.package_type == PackageType::MacOsBundle)
        {
            let app_bundle = bundles.remove(idx);
            for path in &app_bundle.bundle_paths {
                tracing::info!("Cleaning up intermediate .app: {}", path.display());
                if path.is_dir() {
                    let _ = std::fs::remove_dir_all(path);
                } else {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    for bundle in &bundles {
        for path in &bundle.bundle_paths {
            tracing::info!("Bundled: {}", path.display());
        }
    }

    Ok(bundles)
}

impl PackageType {
    /// Priority for build ordering. Lower = built first.
    /// .app must be built before .dmg, updater comes last.
    pub(crate) fn priority(&self) -> u32 {
        match self {
            PackageType::MacOsBundle
            | PackageType::IosBundle
            | PackageType::WindowsMsi
            | PackageType::Nsis
            | PackageType::Deb
            | PackageType::Rpm
            | PackageType::AppImage => 0,
            PackageType::Dmg => 1,
            PackageType::Updater => 2,
        }
    }
}

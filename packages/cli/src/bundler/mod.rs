mod android;
mod category;
mod context;
mod linux;
mod macos;
mod tools;
mod updater;
mod windows;

pub(crate) use context::BundleContext;

use crate::PackageType;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// A completed bundle with its output paths.
#[derive(Debug)]
pub(crate) struct Bundle {
    pub package_type: PackageType,
    pub bundle_paths: Vec<PathBuf>,
}

impl BundleContext<'_> {
    /// Bundles the project for the configured package types.
    pub(crate) async fn bundle_project(&self) -> Result<Vec<Bundle>> {
        let mut package_types = self.package_types();

        // Sort so dependencies come first (e.g. .app before .dmg)
        package_types.sort_by_key(|ptype| match ptype {
            PackageType::MacOsBundle
            | PackageType::IosBundle
            | PackageType::WindowsMsi
            | PackageType::Nsis
            | PackageType::Deb
            | PackageType::Rpm
            | PackageType::AppImage
            | PackageType::Apk
            | PackageType::Aab => 0,
            PackageType::Dmg => 1,
            PackageType::Updater => 2,
        });

        let mut bundles = Vec::<Bundle>::new();

        for package_type in &package_types {
            // Skip if already built (e.g. DMG already built .app)
            if bundles.iter().any(|b| b.package_type == *package_type) {
                continue;
            }

            let bundle_paths = match package_type {
                PackageType::MacOsBundle => self.bundle_macos_app().await?,
                PackageType::Dmg => {
                    let bundled = self.bundle_macos_dmg(&bundles).await?;
                    if !bundled.app.is_empty() {
                        bundles.push(Bundle {
                            package_type: PackageType::MacOsBundle,
                            bundle_paths: bundled.app,
                        });
                    }
                    bundled.dmg
                }
                PackageType::Deb => self.bundle_linux_deb().await?,
                PackageType::Rpm => self.bundle_linux_rpm().await?,
                PackageType::AppImage => self.bundle_linux_appimage().await?,
                PackageType::WindowsMsi => self.bundle_windows_msi().await?,
                PackageType::Nsis => self.bundle_windows_nsis().await?,
                PackageType::Updater => self.bundle_updater(&bundles).await?,
                PackageType::Apk | PackageType::Aab => self.bundle_android(*package_type).await?,
                PackageType::IosBundle => todo!(),
            };

            bundles.push(Bundle {
                package_type: *package_type,
                bundle_paths,
            });
        }

        // On macOS, clean up .app if only building dmg or updater
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
}

/// Recursively copy a directory tree.
///
/// Preserves symlinks on unix targets and falls back to copying link targets on non-unix.
pub(crate) fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &dest_path)?;
        } else if file_type.is_symlink() {
            #[cfg(unix)]
            {
                let target = std::fs::read_link(&source_path)?;
                std::os::unix::fs::symlink(&target, &dest_path)?;
            }

            #[cfg(not(unix))]
            {
                std::fs::copy(&source_path, &dest_path)?;
            }
        } else {
            std::fs::copy(&source_path, &dest_path)?;
        }
    }

    Ok(())
}

//! Updater bundle creation.
//!
//! Creates zip/tar.gz archives of bundle artifacts for auto-update distribution.

use super::{Bundle, BundleContext};
use crate::PackageType;
use anyhow::{Context, Result};
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

impl BundleContext<'_> {
    /// Repackage previously-built bundle artifacts into updater-friendly archives.
    ///
    /// This step never builds the application itself. Instead, it consumes the
    /// [`Bundle`] records produced earlier in the main bundling pass and wraps those
    /// artifacts into archive formats that are convenient for update distribution.
    ///
    /// Archive mapping:
    /// - macOS `.app` bundles become `.app.tar.gz` archives so the bundle directory
    ///   layout is preserved exactly.
    /// - Windows installers (`.msi` and NSIS `.exe`) become single-file `.zip`
    ///   archives.
    /// - Linux artifacts (`.AppImage` and `.deb`) become single-file `.tar.gz`
    ///   archives.
    ///
    /// All outputs are written to `project_out_directory()/bundle/updater`. This
    /// method assumes the input bundles are already finalized and signed as needed; it
    /// performs no platform-specific mutation beyond wrapping them in the selected
    /// archive container.
    pub(crate) async fn bundle_updater(&self, bundles: &[Bundle]) -> Result<Vec<PathBuf>> {
        let mut updater_paths = Vec::new();
        let output_dir = self.project_out_directory().join("updater");
        std::fs::create_dir_all(&output_dir)?;

        for bundle in bundles {
            match bundle.package_type {
                PackageType::MacOsBundle => {
                    // Create .tar.gz of the .app bundle
                    for app_path in &bundle.bundle_paths {
                        let tar_path = output_dir.join(format!(
                            "{}_{}.app.tar.gz",
                            self.product_name(),
                            self.version_string()
                        ));
                        create_tar_gz(app_path, &tar_path)?;
                        tracing::info!("Created updater archive: {}", tar_path.display());
                        updater_paths.push(tar_path);
                    }
                }
                PackageType::Nsis | PackageType::WindowsMsi => {
                    // Create .zip of the installer
                    for installer_path in &bundle.bundle_paths {
                        let zip_path = output_dir.join(format!(
                            "{}.zip",
                            installer_path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                        ));
                        create_zip(installer_path, &zip_path)?;
                        tracing::info!("Created updater archive: {}", zip_path.display());
                        updater_paths.push(zip_path);
                    }
                }
                PackageType::AppImage | PackageType::Deb => {
                    // Create .tar.gz of the artifact
                    for artifact_path in &bundle.bundle_paths {
                        let tar_path = output_dir.join(format!(
                            "{}.tar.gz",
                            artifact_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        ));
                        create_tar_gz_single_file(artifact_path, &tar_path)?;
                        tracing::info!("Created updater archive: {}", tar_path.display());
                        updater_paths.push(tar_path);
                    }
                }
                _ => {}
            }
        }

        Ok(updater_paths)
    }
}

/// Create a .tar.gz of a directory (e.g., a .app bundle).
fn create_tar_gz(src_dir: &Path, dest: &Path) -> Result<()> {
    let file =
        File::create(dest).with_context(|| format!("Failed to create {}", dest.display()))?;
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    let dir_name = src_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    tar.append_dir_all(&dir_name, src_dir)
        .with_context(|| format!("Failed to add {} to tar", src_dir.display()))?;

    tar.into_inner()?.finish()?;
    Ok(())
}

/// Create a .tar.gz containing a single file.
fn create_tar_gz_single_file(src_file: &Path, dest: &Path) -> Result<()> {
    let file =
        File::create(dest).with_context(|| format!("Failed to create {}", dest.display()))?;
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    let file_name = src_file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    tar.append_path_with_name(src_file, &file_name)
        .with_context(|| format!("Failed to add {} to tar", src_file.display()))?;

    tar.into_inner()?.finish()?;
    Ok(())
}

/// Create a .zip containing a single file.
fn create_zip(src_file: &Path, dest: &Path) -> Result<()> {
    let file =
        File::create(dest).with_context(|| format!("Failed to create {}", dest.display()))?;
    let mut zip = zip::ZipWriter::new(file);

    let file_name = src_file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(&file_name, options)?;
    let mut src = File::open(src_file)?;
    io::copy(&mut src, &mut zip)?;

    zip.finish()?;
    Ok(())
}

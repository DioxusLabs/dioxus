//! AppImage bundler.
//!
//! Creates AppImage bundles using the linuxdeploy tool.
//! The AppDir structure is built from the same data layout as Debian packages,
//! then linuxdeploy processes it into a self-contained AppImage.

use super::{debian, freedesktop};
use crate::bundler::{context::Arch, tools, BundleContext};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Bundle the project as an AppImage.
///
/// Returns the list of created .AppImage file paths.
pub(crate) fn bundle_project(ctx: &BundleContext) -> Result<Vec<PathBuf>> {
    let name = ctx.main_binary_name().to_string();
    let version = ctx.version_string();
    let arch = ctx.binary_arch();
    let arch_str = appimage_arch(arch);

    let output_dir = ctx
        .project_out_directory()
        .join("bundle")
        .join("appimage");
    fs::create_dir_all(&output_dir)?;

    let appimage_filename = format!("{name}_{version}_{arch_str}.AppImage");
    let appimage_path = output_dir.join(&appimage_filename);

    tracing::info!("Bundling {appimage_filename}...");

    // Set up the AppDir
    let appdir = output_dir.join(format!("{name}.AppDir"));
    if appdir.exists() {
        fs::remove_dir_all(&appdir)?;
    }
    fs::create_dir_all(&appdir)?;

    // Generate the data directory structure (reuse Debian's data generation)
    debian::generate_data(&appdir, ctx)?;

    // Create the AppRun symlink -> usr/bin/{binary}
    create_appdir_symlinks(&appdir, &name, ctx)?;

    // Ensure linuxdeploy is available
    let linuxdeploy_arch = linuxdeploy_arch(arch);
    let linuxdeploy = tools::ensure_linuxdeploy(&ctx.tools_dir(), linuxdeploy_arch)?;

    // Run linuxdeploy to create the AppImage
    tracing::info!("Running linuxdeploy...");

    // linuxdeploy needs the OUTPUT env var to control where the AppImage is written
    let status = Command::new(&linuxdeploy)
        .arg("--appdir")
        .arg(&appdir)
        .arg("--output")
        .arg("appimage")
        .env("OUTPUT", &appimage_path)
        // Prevent linuxdeploy from trying to modify the binary with patchelf when not needed
        .env("NO_STRIP", "true")
        .current_dir(&output_dir)
        .status()
        .with_context(|| {
            format!(
                "Failed to run linuxdeploy: {}",
                linuxdeploy.display()
            )
        })?;

    if !status.success() {
        bail!(
            "linuxdeploy failed with exit code: {}",
            status.code().unwrap_or(-1)
        );
    }

    if !appimage_path.exists() {
        // linuxdeploy might have written the file with its own naming convention.
        // Look for any .AppImage file in the output directory.
        let found = find_appimage_output(&output_dir, &name)?;
        if let Some(found_path) = found {
            if found_path != appimage_path {
                fs::rename(&found_path, &appimage_path).with_context(|| {
                    format!(
                        "Failed to rename {} to {}",
                        found_path.display(),
                        appimage_path.display()
                    )
                })?;
            }
        } else {
            bail!(
                "AppImage was not created. Expected at: {}",
                appimage_path.display()
            );
        }
    }

    // Clean up the AppDir
    let _ = fs::remove_dir_all(&appdir);

    tracing::info!("Created AppImage: {}", appimage_path.display());
    Ok(vec![appimage_path])
}

/// Create the top-level symlinks in the AppDir that AppImage/linuxdeploy expects:
/// - `AppRun` -> `usr/bin/{binary}`
/// - `{name}.desktop` -> `usr/share/applications/{name}.desktop`
/// - `{name}.png` -> `usr/share/icons/hicolor/{largest}/apps/{name}.png`
fn create_appdir_symlinks(appdir: &Path, name: &str, ctx: &BundleContext) -> Result<()> {
    // AppRun symlink
    let apprun = appdir.join("AppRun");
    let bin_target = format!("usr/bin/{name}");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&bin_target, &apprun).with_context(|| {
            format!(
                "Failed to create AppRun symlink: {} -> {}",
                apprun.display(),
                bin_target
            )
        })?;
    }

    #[cfg(not(unix))]
    {
        // On non-unix, just copy the binary
        let src = appdir.join(&bin_target);
        if src.exists() {
            fs::copy(&src, &apprun)?;
        }
    }

    // Desktop file symlink
    let desktop_link = appdir.join(format!("{name}.desktop"));
    let desktop_target = format!("usr/share/applications/{name}.desktop");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&desktop_target, &desktop_link).with_context(|| {
            format!(
                "Failed to create desktop symlink: {} -> {}",
                desktop_link.display(),
                desktop_target
            )
        })?;
    }

    #[cfg(not(unix))]
    {
        let src = appdir.join(&desktop_target);
        if src.exists() {
            fs::copy(&src, &desktop_link)?;
        }
    }

    // Icon symlink - find the largest PNG icon
    if let Some(largest_icon) = freedesktop::find_largest_icon(ctx)? {
        let icon_link = appdir.join(format!("{name}.png"));

        // Find where this icon was copied to in the AppDir hicolor directory
        if let Some(icon_in_appdir) = find_icon_in_appdir(appdir, name) {
            let relative = icon_in_appdir
                .strip_prefix(appdir)
                .unwrap_or(&icon_in_appdir);
            let relative_str = relative.to_string_lossy().to_string();

            #[cfg(unix)]
            std::os::unix::fs::symlink(&relative_str, &icon_link).with_context(|| {
                format!(
                    "Failed to create icon symlink: {} -> {}",
                    icon_link.display(),
                    relative_str
                )
            })?;

            #[cfg(not(unix))]
            fs::copy(&icon_in_appdir, &icon_link)?;
        } else {
            // No icon was found in the AppDir, copy the source icon directly
            fs::copy(&largest_icon, &icon_link).with_context(|| {
                format!(
                    "Failed to copy icon {} to AppDir",
                    largest_icon.display()
                )
            })?;
        }
    }

    Ok(())
}

/// Find the icon file within the AppDir's hicolor directory.
/// Returns the path to the largest PNG icon found.
fn find_icon_in_appdir(appdir: &Path, name: &str) -> Option<PathBuf> {
    let icons_dir = appdir.join("usr/share/icons/hicolor");
    if !icons_dir.exists() {
        return None;
    }

    let mut best: Option<(u32, PathBuf)> = None;
    let target_name = format!("{name}.png");

    if let Ok(entries) = fs::read_dir(&icons_dir) {
        for entry in entries.flatten() {
            let size_dir = entry.path();
            let icon_path = size_dir.join("apps").join(&target_name);
            if icon_path.exists() {
                // Parse size from directory name like "128x128"
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if let Some(size_str) = dir_name.split('x').next() {
                    if let Ok(size) = size_str.parse::<u32>() {
                        if best.as_ref().map_or(true, |(best_size, _)| size > *best_size) {
                            best = Some((size, icon_path));
                        }
                    }
                }
            }
        }
    }

    best.map(|(_, path)| path)
}

/// Search for an .AppImage file in the output directory.
fn find_appimage_output(dir: &Path, name: &str) -> Result<Option<PathBuf>> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if file_name.ends_with(".AppImage") && file_name.contains(name) {
                    return Ok(Some(path));
                }
            }
        }
    }
    Ok(None)
}

/// Map Arch to the architecture string used in AppImage filenames.
fn appimage_arch(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "x86_64",
        Arch::X86 => "i386",
        Arch::AArch64 => "aarch64",
        Arch::Armhf => "armhf",
        Arch::Armel => "armel",
        Arch::Riscv64 => "riscv64",
        Arch::Universal => "x86_64",
    }
}

/// Map Arch to the architecture string used in linuxdeploy binary names.
fn linuxdeploy_arch(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "x86_64",
        Arch::X86 => "i386",
        Arch::AArch64 => "aarch64",
        Arch::Armhf => "armhf",
        Arch::Armel => "armhf",
        Arch::Riscv64 => "x86_64", // fallback: linuxdeploy may not have riscv64 builds
        Arch::Universal => "x86_64",
    }
}

//! RPM package bundler.
//!
//! Creates .rpm packages using the `rpm` crate.

use super::freedesktop;
use crate::bundler::{context::Arch, BundleContext};
use anyhow::{Context, Result};
use std::{
    fs,
    io::BufReader,
    path::{Path, PathBuf},
};

/// Bundle the project as an RPM package.
///
/// RPM payload layout mirrors the Linux desktop package conventions:
/// - `/usr/bin/<binary>` main executable
/// - `/usr/lib/<binary>/...` application resources
/// - `/usr/share/applications/<binary>.desktop`
/// - `/usr/share/icons/hicolor/...` icons
///
/// Additional metadata and lifecycle scripts are populated from bundle settings
/// (license, dependencies, pre/post install/remove scripts).
///
/// Returns the list of created `.rpm` file paths.
pub(crate) async fn bundle_project(ctx: &BundleContext<'_>) -> Result<Vec<PathBuf>> {
    let name = ctx.main_binary_name().to_string();
    let version = ctx.version_string();
    let arch = rpm_arch(ctx.binary_arch());
    let license = ctx.license().unwrap_or("Unknown").to_string();
    let description = ctx.short_description();

    let output_dir = ctx.project_out_directory().join("bundle").join("rpm");
    fs::create_dir_all(&output_dir)?;

    let rpm_filename = format!("{name}-{version}-1.{arch}.rpm");
    let rpm_path = output_dir.join(&rpm_filename);

    tracing::info!("Bundling {rpm_filename}...");

    // Start building the RPM package
    let mut builder = rpm::PackageBuilder::new(&name, &version, &license, arch, &description)
        .using_config(rpm::BuildConfig::v4().compression(rpm::CompressionType::Gzip));

    // Add the main binary
    let binary_path = ctx.main_binary_path();
    let dest_bin = format!("/usr/bin/{name}");
    builder = builder
        .with_file(&binary_path, rpm::FileOptions::new(dest_bin).mode(0o755))
        .context("Failed to add binary to RPM")?;

    // Generate and add the .desktop file
    let deb_settings = ctx.deb();
    let desktop_content =
        freedesktop::generate_desktop_file(ctx, deb_settings.desktop_template.as_deref())?;
    let desktop_dest = format!("/usr/share/applications/{name}.desktop");

    // Write desktop file to a temporary location so we can add it
    let temp_dir = output_dir.join("_rpm_temp");
    fs::create_dir_all(&temp_dir)?;
    let temp_desktop = temp_dir.join(format!("{name}.desktop"));
    fs::write(&temp_desktop, &desktop_content)?;

    builder = builder
        .with_file(
            &temp_desktop,
            rpm::FileOptions::new(desktop_dest).mode(0o644),
        )
        .context("Failed to add desktop file to RPM")?;

    // Add icon files
    let icon_files = ctx.icon_files()?;
    for icon_path in &icon_files {
        let ext = icon_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "png" => {
                if let Ok(file) = fs::File::open(icon_path) {
                    let decoder = png::Decoder::new(BufReader::new(file));
                    if let Ok(reader) = decoder.read_info() {
                        let info = reader.info();
                        let (w, h) = (info.width, info.height);
                        let size = w.max(h);
                        let dest =
                            format!("/usr/share/icons/hicolor/{size}x{size}/apps/{name}.png");
                        builder = builder
                            .with_file(icon_path, rpm::FileOptions::new(dest).mode(0o644))
                            .context("Failed to add icon to RPM")?;
                    }
                }
            }
            "svg" => {
                let dest = format!("/usr/share/icons/hicolor/scalable/apps/{name}.svg");
                builder = builder
                    .with_file(icon_path, rpm::FileOptions::new(dest).mode(0o644))
                    .context("Failed to add SVG icon to RPM")?;
            }
            _ => {
                tracing::warn!(
                    "Skipping icon with unsupported extension '{}': {}",
                    ext,
                    icon_path.display()
                );
            }
        }
    }

    // Add resources: copy to temp dir, then collect file paths and add them
    let resource_temp = temp_dir.join("resources");
    fs::create_dir_all(&resource_temp)?;
    ctx.copy_resources(&resource_temp)?;

    let resource_files = collect_files(&resource_temp)?;
    for (src, relative) in &resource_files {
        let dest = format!(
            "/usr/lib/{name}/{}",
            relative.to_string_lossy().replace('\\', "/")
        );
        builder = builder
            .with_file(src, rpm::FileOptions::new(&dest).mode(0o644))
            .with_context(|| format!("Failed to add resource {} to RPM", relative.display()))?;
    }

    // Add custom files from deb settings (reused for RPM)
    let crate_dir = ctx.crate_dir();
    for (dest_path, src_path) in &deb_settings.files {
        let src = if src_path.is_absolute() {
            src_path.clone()
        } else {
            crate_dir.join(src_path)
        };
        if src.exists() {
            let dest = dest_path.to_string_lossy().to_string();
            let dest = if dest.starts_with('/') {
                dest
            } else {
                format!("/{dest}")
            };
            builder = builder
                .with_file(&src, rpm::FileOptions::new(&dest).mode(0o644))
                .context("Failed to add custom file to RPM")?;
        }
    }

    // Add maintainer scripts
    if let Some(script_path) = &deb_settings.pre_install_script {
        let path = resolve_path(&crate_dir, script_path);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read pre-install script: {}", path.display()))?;
        builder = builder.pre_install_script(content);
    }
    if let Some(script_path) = &deb_settings.post_install_script {
        let path = resolve_path(&crate_dir, script_path);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read post-install script: {}", path.display()))?;
        builder = builder.post_install_script(content);
    }
    if let Some(script_path) = &deb_settings.pre_remove_script {
        let path = resolve_path(&crate_dir, script_path);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read pre-uninstall script: {}", path.display()))?;
        builder = builder.pre_uninstall_script(content);
    }
    if let Some(script_path) = &deb_settings.post_remove_script {
        let path = resolve_path(&crate_dir, script_path);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read post-uninstall script: {}", path.display()))?;
        builder = builder.post_uninstall_script(content);
    }

    // Add dependencies
    if let Some(deps) = &deb_settings.depends {
        for dep in deps {
            builder = builder.requires(rpm::Dependency::any(dep));
        }
    }

    // Build the RPM
    let package = builder.build().context("Failed to build RPM package")?;

    // Write to file
    let mut rpm_file = fs::File::create(&rpm_path)
        .with_context(|| format!("Failed to create {}", rpm_path.display()))?;
    package
        .write(&mut rpm_file)
        .context("Failed to write RPM package")?;

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir);

    tracing::info!("Created RPM package: {}", rpm_path.display());
    Ok(vec![rpm_path])
}

/// Collect all files in a directory, returning (absolute_path, relative_path) pairs.
fn collect_files(dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let abs = entry.path().to_path_buf();
        let rel = entry
            .path()
            .strip_prefix(dir)
            .unwrap_or(entry.path())
            .to_path_buf();
        files.push((abs, rel));
    }
    Ok(files)
}

/// Map Arch to RPM architecture string.
fn rpm_arch(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "x86_64",
        Arch::X86 => "i686",
        Arch::AArch64 => "aarch64",
        Arch::Armhf => "armv7hl",
        Arch::Armel => "armv6l",
        Arch::Riscv64 => "riscv64",
        Arch::Universal => "noarch",
    }
}

/// Resolve a path that may be relative to the crate directory.
fn resolve_path(crate_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        crate_dir.join(path)
    }
}

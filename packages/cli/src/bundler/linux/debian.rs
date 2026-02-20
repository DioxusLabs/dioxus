//! Debian .deb package bundler.
//!
//! Creates .deb packages using pure Rust (ar, tar, flate2 crates).
//! A .deb is an AR archive containing:
//! 1. `debian-binary` - version string "2.0\n"
//! 2. `control.tar.gz` - package metadata (control file, md5sums, maintainer scripts)
//! 3. `data.tar.gz` - the actual installed files

use super::freedesktop;
use crate::bundler::{context::Arch, BundleContext};
use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

/// Bundle the project as a .deb package.
///
/// Returns the list of created .deb file paths.
pub(crate) fn bundle_project(ctx: &BundleContext) -> Result<Vec<PathBuf>> {
    let arch = deb_arch(ctx.binary_arch());
    let package_name = deb_package_name(ctx);
    let version = ctx.version_string();

    let output_dir = ctx
        .project_out_directory()
        .join("bundle")
        .join("deb");
    fs::create_dir_all(&output_dir)?;

    let deb_filename = format!("{package_name}_{version}_{arch}.deb");
    let deb_path = output_dir.join(&deb_filename);

    tracing::info!("Bundling {deb_filename}...");

    // Build the data directory tree in a temp location
    let data_dir = output_dir.join("_data");
    if data_dir.exists() {
        fs::remove_dir_all(&data_dir)?;
    }
    fs::create_dir_all(&data_dir)?;

    generate_data(&data_dir, ctx)?;

    // Calculate installed size (in KB)
    let installed_size = dir_size_kb(&data_dir)?;

    // Build control.tar.gz
    let control_tar = build_control_tar(ctx, &package_name, &version, arch, installed_size, &data_dir)?;

    // Build data.tar.gz
    let data_tar = build_data_tar(&data_dir)?;

    // Assemble the .deb AR archive
    let deb_file = File::create(&deb_path)
        .with_context(|| format!("Failed to create {}", deb_path.display()))?;
    let mut ar_builder = ar::Builder::new(deb_file);

    // 1. debian-binary
    let debian_binary = b"2.0\n";
    let mut header = ar::Header::new(b"debian-binary".to_vec(), debian_binary.len() as u64);
    header.set_mode(0o100644);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    ar_builder.append(&header, &debian_binary[..])?;

    // 2. control.tar.gz
    let mut header = ar::Header::new(b"control.tar.gz".to_vec(), control_tar.len() as u64);
    header.set_mode(0o100644);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    ar_builder.append(&header, control_tar.as_slice())?;

    // 3. data.tar.gz
    let mut header = ar::Header::new(b"data.tar.gz".to_vec(), data_tar.len() as u64);
    header.set_mode(0o100644);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    ar_builder.append(&header, data_tar.as_slice())?;

    // Clean up temp data directory
    let _ = fs::remove_dir_all(&data_dir);

    tracing::info!("Created deb package: {}", deb_path.display());
    Ok(vec![deb_path])
}

/// Generate the data directory tree for the .deb package.
/// This is also reused by appimage for its AppDir structure.
pub(crate) fn generate_data(data_dir: &Path, ctx: &BundleContext) -> Result<()> {
    let bin_name = ctx.main_binary_name();

    // usr/bin/{binary}
    let bin_dir = data_dir.join("usr/bin");
    fs::create_dir_all(&bin_dir)?;
    let bin_dest = bin_dir.join(bin_name);
    fs::copy(ctx.main_binary_path(), &bin_dest)
        .with_context(|| format!("Failed to copy binary to {}", bin_dest.display()))?;

    // Set executable permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bin_dest, fs::Permissions::from_mode(0o755))?;
    }

    // usr/share/applications/{name}.desktop
    let desktop_dir = data_dir.join("usr/share/applications");
    fs::create_dir_all(&desktop_dir)?;

    let deb_settings = ctx.deb();
    let desktop_content = freedesktop::generate_desktop_file(
        ctx,
        deb_settings.desktop_template.as_deref(),
    )?;
    let desktop_path = desktop_dir.join(format!("{bin_name}.desktop"));
    fs::write(&desktop_path, &desktop_content)?;

    // usr/share/icons/hicolor/{size}x{size}/apps/{name}.png
    freedesktop::copy_icons(ctx, data_dir)?;

    // usr/lib/{name}/ (resources)
    let resource_dir = data_dir.join(format!("usr/lib/{bin_name}"));
    fs::create_dir_all(&resource_dir)?;
    ctx.copy_resources(&resource_dir)?;

    // Copy external binaries
    let ext_bin_dir = data_dir.join("usr/bin");
    ctx.copy_external_binaries(&ext_bin_dir)?;

    // Custom deb files
    for (deb_path, src_path) in &deb_settings.files {
        let dest = data_dir.join(deb_path.strip_prefix("/").unwrap_or(deb_path));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let src = if src_path.is_absolute() {
            src_path.clone()
        } else {
            ctx.crate_dir().join(src_path)
        };
        fs::copy(&src, &dest).with_context(|| {
            format!(
                "Failed to copy custom deb file {} -> {}",
                src.display(),
                dest.display()
            )
        })?;
    }

    // Changelog
    if let Some(changelog_path) = &deb_settings.changelog {
        let changelog_src = if changelog_path.is_absolute() {
            changelog_path.clone()
        } else {
            ctx.crate_dir().join(changelog_path)
        };
        if changelog_src.exists() {
            let doc_dir = data_dir.join(format!("usr/share/doc/{bin_name}"));
            fs::create_dir_all(&doc_dir)?;
            let changelog_content = fs::read(&changelog_src)?;
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
            encoder.write_all(&changelog_content)?;
            let compressed = encoder.finish()?;
            fs::write(doc_dir.join("changelog.gz"), compressed)?;
        }
    }

    Ok(())
}

/// Build the control.tar.gz containing the control file, md5sums, and maintainer scripts.
fn build_control_tar(
    ctx: &BundleContext,
    package_name: &str,
    version: &str,
    arch: &str,
    installed_size: u64,
    data_dir: &Path,
) -> Result<Vec<u8>> {
    let buf = Vec::new();
    let encoder = flate2::write::GzEncoder::new(buf, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);

    // Generate the control file
    let control = generate_control_file(ctx, package_name, version, arch, installed_size)?;
    append_tar_bytes(&mut tar, "./control", control.as_bytes(), 0o644)?;

    // Generate md5sums
    let md5sums = generate_md5sums(data_dir)?;
    append_tar_bytes(&mut tar, "./md5sums", md5sums.as_bytes(), 0o644)?;

    // Add maintainer scripts
    let deb = ctx.deb();
    let crate_dir = ctx.crate_dir();

    if let Some(script_path) = &deb.pre_install_script {
        let path = resolve_path(&crate_dir, script_path);
        let content = fs::read(&path)
            .with_context(|| format!("Failed to read preinst script: {}", path.display()))?;
        append_tar_bytes(&mut tar, "./preinst", &content, 0o755)?;
    }
    if let Some(script_path) = &deb.post_install_script {
        let path = resolve_path(&crate_dir, script_path);
        let content = fs::read(&path)
            .with_context(|| format!("Failed to read postinst script: {}", path.display()))?;
        append_tar_bytes(&mut tar, "./postinst", &content, 0o755)?;
    }
    if let Some(script_path) = &deb.pre_remove_script {
        let path = resolve_path(&crate_dir, script_path);
        let content = fs::read(&path)
            .with_context(|| format!("Failed to read prerm script: {}", path.display()))?;
        append_tar_bytes(&mut tar, "./prerm", &content, 0o755)?;
    }
    if let Some(script_path) = &deb.post_remove_script {
        let path = resolve_path(&crate_dir, script_path);
        let content = fs::read(&path)
            .with_context(|| format!("Failed to read postrm script: {}", path.display()))?;
        append_tar_bytes(&mut tar, "./postrm", &content, 0o755)?;
    }

    let encoder = tar.into_inner()?;
    let data = encoder.finish()?;
    Ok(data)
}

/// Generate the Debian control file content.
fn generate_control_file(
    ctx: &BundleContext,
    package_name: &str,
    version: &str,
    arch: &str,
    installed_size: u64,
) -> Result<String> {
    let deb = ctx.deb();

    let mut control = String::new();
    control.push_str(&format!("Package: {package_name}\n"));
    control.push_str(&format!("Version: {version}\n"));
    control.push_str(&format!("Architecture: {arch}\n"));
    control.push_str(&format!("Installed-Size: {installed_size}\n"));

    // Description
    let description = ctx.short_description();
    if !description.is_empty() {
        control.push_str(&format!("Description: {description}\n"));

        // Long description is indented with a space per line under Description
        if let Some(long_desc) = ctx.long_description() {
            for line in long_desc.lines() {
                if line.is_empty() {
                    control.push_str(" .\n");
                } else {
                    control.push_str(&format!(" {line}\n"));
                }
            }
        }
    }

    // Section
    let section = deb.section.as_deref().unwrap_or("utils");
    control.push_str(&format!("Section: {section}\n"));

    // Priority
    let priority = deb.priority.as_deref().unwrap_or("optional");
    control.push_str(&format!("Priority: {priority}\n"));

    // Homepage
    if let Some(url) = ctx.homepage_url() {
        control.push_str(&format!("Homepage: {url}\n"));
    }

    // Maintainer
    let maintainer = ctx
        .authors_comma_separated()
        .unwrap_or_else(|| "Unknown".to_string());
    control.push_str(&format!("Maintainer: {maintainer}\n"));

    // Depends
    if let Some(deps) = &deb.depends {
        if !deps.is_empty() {
            control.push_str(&format!("Depends: {}\n", deps.join(", ")));
        }
    }

    // Recommends
    if let Some(recs) = &deb.recommends {
        if !recs.is_empty() {
            control.push_str(&format!("Recommends: {}\n", recs.join(", ")));
        }
    }

    // Provides
    if let Some(provs) = &deb.provides {
        if !provs.is_empty() {
            control.push_str(&format!("Provides: {}\n", provs.join(", ")));
        }
    }

    // Conflicts
    if let Some(conflicts) = &deb.conflicts {
        if !conflicts.is_empty() {
            control.push_str(&format!("Conflicts: {}\n", conflicts.join(", ")));
        }
    }

    // Replaces
    if let Some(replaces) = &deb.replaces {
        if !replaces.is_empty() {
            control.push_str(&format!("Replaces: {}\n", replaces.join(", ")));
        }
    }

    Ok(control)
}

/// Build data.tar.gz from the data directory.
fn build_data_tar(data_dir: &Path) -> Result<Vec<u8>> {
    let buf = Vec::new();
    let encoder = flate2::write::GzEncoder::new(buf, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);

    // Walk the data directory and add all files
    tar.append_dir_all(".", data_dir)
        .context("Failed to build data.tar.gz")?;

    let encoder = tar.into_inner()?;
    let data = encoder.finish()?;
    Ok(data)
}

/// Generate md5sums file for all files in the data directory.
/// Format: "{md5hash}  {relative_path}\n" for each file.
fn generate_md5sums(data_dir: &Path) -> Result<String> {
    let mut md5sums = String::new();

    for entry in walkdir::WalkDir::new(data_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let content = fs::read(path)?;
        let digest = md5::compute(&content);
        let relative = path.strip_prefix(data_dir).unwrap_or(path);
        let relative_str = relative.to_string_lossy().replace('\\', "/");

        md5sums.push_str(&format!("{:x}  {relative_str}\n", digest));
    }

    Ok(md5sums)
}

/// Append raw bytes as a file entry in a tar archive.
fn append_tar_bytes<W: Write>(
    tar: &mut tar::Builder<W>,
    path: &str,
    data: &[u8],
    mode: u32,
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_path(path)?;
    header.set_size(data.len() as u64);
    header.set_mode(mode);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header.set_cksum();

    tar.append(&header, Cursor::new(data))
        .with_context(|| format!("Failed to add {path} to tar"))?;

    Ok(())
}

/// Map Arch enum to Debian architecture string.
fn deb_arch(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "amd64",
        Arch::X86 => "i386",
        Arch::AArch64 => "arm64",
        Arch::Armhf => "armhf",
        Arch::Armel => "armel",
        Arch::Riscv64 => "riscv64",
        Arch::Universal => "all",
    }
}

/// Generate a Debian-friendly package name (lowercase, hyphens instead of underscores).
fn deb_package_name(ctx: &BundleContext) -> String {
    ctx.main_binary_name()
        .to_lowercase()
        .replace('_', "-")
}

/// Calculate total size of a directory tree in kilobytes.
fn dir_size_kb(path: &Path) -> Result<u64> {
    let mut total: u64 = 0;
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }
    // Round up to nearest KB
    Ok((total + 1023) / 1024)
}

/// Resolve a path that may be relative to the crate directory.
fn resolve_path(crate_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        crate_dir.join(path)
    }
}

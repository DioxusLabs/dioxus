use crate::bundler::macos::{app, sign};
use crate::bundler::{Bundle, BundleContext};
use crate::PackageType;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The result of DMG bundling, which may include both the `.dmg` and `.app` outputs.
pub(crate) struct DmgBundled {
    /// Paths to the generated `.dmg` file(s).
    pub dmg: Vec<PathBuf>,
    /// Paths to the generated `.app` bundle(s) (if the `.app` was built as a dependency).
    pub app: Vec<PathBuf>,
}

/// Bundle the project as a `.dmg` disk image.
///
/// If the `.app` bundle has not already been created (not present in `bundles`),
/// it will be built first. The `.app` is then packaged into a `.dmg` using `hdiutil`.
pub(crate) fn bundle_project(ctx: &BundleContext, bundles: &[Bundle]) -> Result<DmgBundled> {
    let product_name = ctx.product_name();
    let macos_settings = ctx.macos();

    let bundle_name = macos_settings
        .bundle_name
        .as_deref()
        .unwrap_or(&product_name);

    // Check if the .app bundle already exists from a previous step
    let (app_paths, app_bundle_paths) = if let Some(app_bundle) = bundles
        .iter()
        .find(|b| b.package_type == PackageType::MacOsBundle)
    {
        (app_bundle.bundle_paths.clone(), Vec::new())
    } else {
        // Build the .app bundle first
        let paths = app::bundle_project(ctx)?;
        (paths.clone(), paths)
    };

    if app_paths.is_empty() {
        bail!("No .app bundle found to package into a DMG");
    }

    let app_path = &app_paths[0];
    if !app_path.exists() {
        bail!(
            ".app bundle does not exist at expected path: {}",
            app_path.display()
        );
    }

    let output_dir = ctx.project_out_directory().join("macos");
    fs::create_dir_all(&output_dir)?;

    let dmg_filename = format!(
        "{}_{}_{}",
        bundle_name,
        ctx.version_string(),
        ctx.binary_arch()
    );
    let dmg_path = output_dir.join(format!("{dmg_filename}.dmg"));

    tracing::info!("Creating DMG at {}", dmg_path.display());

    // Create a temporary directory for the DMG contents
    let staging_dir = tempfile::tempdir().context("Failed to create temp dir for DMG staging")?;
    let staging_path = staging_dir.path();

    // Copy the .app bundle into the staging directory
    let staged_app = staging_path.join(app_path.file_name().unwrap());
    copy_dir_recursive(app_path, &staged_app)?;

    // Create a symlink to /Applications for drag-and-drop install
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/Applications", staging_path.join("Applications"))
            .context("Failed to create /Applications symlink in DMG staging")?;
    }

    // Remove any existing DMG at the output path
    if dmg_path.exists() {
        fs::remove_file(&dmg_path)?;
    }

    // Create the DMG using hdiutil
    let status = Command::new("hdiutil")
        .args([
            "create",
            "-volname",
            bundle_name,
            "-srcfolder",
            &staging_path.display().to_string(),
            "-ov",
            "-format",
            "UDZO",
            &dmg_path.display().to_string(),
        ])
        .status()
        .context("Failed to execute `hdiutil create`")?;

    if !status.success() {
        bail!("`hdiutil create` failed with exit code: {status}");
    }

    tracing::info!("DMG created at {}", dmg_path.display());

    // Sign the DMG if a signing identity is available
    let signing_identity = sign::setup_keychain(macos_settings.signing_identity.as_deref())?;
    if let Some(identity) = &signing_identity {
        tracing::info!("Signing DMG with identity: {}", identity.identity);
        sign::sign_paths(
            identity,
            vec![sign::SignTarget {
                path: dmg_path.clone(),
            }],
            &macos_settings,
        )?;

        // Notarize the DMG if credentials are available
        let should_notarize =
            std::env::var("APPLE_ID").is_ok() || std::env::var("APPLE_API_KEY").is_ok();

        if should_notarize {
            sign::notarize(&dmg_path)?;
        }
    }

    Ok(DmgBundled {
        dmg: vec![dmg_path],
        app: app_bundle_paths,
    })
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_dest = dest.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &entry_dest)?;
        } else if file_type.is_symlink() {
            #[cfg(unix)]
            {
                let target = fs::read_link(entry.path())?;
                std::os::unix::fs::symlink(&target, &entry_dest)?;
            }
        } else {
            fs::copy(entry.path(), &entry_dest)?;
        }
    }
    Ok(())
}

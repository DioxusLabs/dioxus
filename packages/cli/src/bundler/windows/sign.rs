//! Windows code signing support.
//!
//! Provides functions for signing Windows binaries using either signtool.exe
//! or a custom signing command specified in the bundle configuration.

use crate::bundler::BundleContext;
use crate::WindowsSettings;
use anyhow::{bail, Context, Result};
use std::path::Path;

/// Returns `true` if the Windows settings have signing configured.
pub(crate) fn can_sign(settings: &WindowsSettings) -> bool {
    settings.certificate_thumbprint.is_some() || settings.sign_command.is_some()
}

/// Attempt to sign a binary at the given path using the signing configuration
/// from the BundleContext's Windows settings.
///
/// If no signing configuration is present, this is a no-op.
pub(crate) fn try_sign(path: &Path, ctx: &BundleContext) -> Result<()> {
    let settings = ctx.windows();

    if !can_sign(&settings) {
        return Ok(());
    }

    tracing::info!("Signing {}", path.display());

    // Custom sign command takes priority
    if let Some(sign_cmd) = &settings.sign_command {
        return run_custom_sign_command(path, &sign_cmd.cmd, &sign_cmd.args);
    }

    // Otherwise use signtool with certificate thumbprint
    if let Some(thumbprint) = &settings.certificate_thumbprint {
        return run_signtool_sign(path, thumbprint, &settings);
    }

    Ok(())
}

/// Run a custom signing command. The `%1` placeholder in args is replaced
/// with the path to the binary to sign.
fn run_custom_sign_command(path: &Path, cmd: &str, args: &[String]) -> Result<()> {
    let path_str = path.to_string_lossy();
    let resolved_args: Vec<String> = args
        .iter()
        .map(|arg| arg.replace("%1", &path_str))
        .collect();

    tracing::debug!("Running custom sign command: {} {:?}", cmd, resolved_args);

    let status = std::process::Command::new(cmd)
        .args(&resolved_args)
        .status()
        .with_context(|| format!("Failed to run custom sign command: {cmd}"))?;

    if !status.success() {
        bail!(
            "Custom sign command failed with exit code: {:?}",
            status.code()
        );
    }

    Ok(())
}

/// Run signtool.exe to sign a binary with a certificate thumbprint.
///
/// This only works on Windows where signtool.exe is available.
#[cfg(target_os = "windows")]
fn run_signtool_sign(
    path: &Path,
    thumbprint: &str,
    settings: &WindowsSettings,
) -> Result<()> {
    let mut args = vec![
        "sign".to_string(),
        "/fd".to_string(),
        settings
            .digest_algorithm
            .clone()
            .unwrap_or_else(|| "sha256".to_string()),
        "/sha1".to_string(),
        thumbprint.to_string(),
    ];

    if let Some(timestamp_url) = &settings.timestamp_url {
        if settings.tsp {
            args.push("/tr".to_string());
            args.push(timestamp_url.clone());
            args.push("/td".to_string());
            args.push(
                settings
                    .digest_algorithm
                    .clone()
                    .unwrap_or_else(|| "sha256".to_string()),
            );
        } else {
            args.push("/t".to_string());
            args.push(timestamp_url.clone());
        }
    }

    args.push(path.to_string_lossy().to_string());

    tracing::debug!("Running signtool with args: {:?}", args);

    let status = std::process::Command::new("signtool.exe")
        .args(&args)
        .status()
        .context("Failed to run signtool.exe. Is the Windows SDK installed?")?;

    if !status.success() {
        bail!(
            "signtool.exe failed with exit code: {:?}",
            status.code()
        );
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_signtool_sign(
    _path: &Path,
    _thumbprint: &str,
    _settings: &WindowsSettings,
) -> Result<()> {
    tracing::warn!("signtool.exe signing is only available on Windows. Skipping signing.");
    Ok(())
}


//! Windows bundler utility functions.
//!
//! Constants and helpers for NSIS and MSI bundling, including
//! WebView2 runtime downloading.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Output folder name for NSIS installers within the bundle directory.
pub(crate) const NSIS_OUTPUT_FOLDER_NAME: &str = "nsis";

/// Output folder name for MSI installers within the bundle directory.
pub(crate) const WIX_OUTPUT_FOLDER_NAME: &str = "msi";

/// WebView2 bootstrapper download URL.
const WEBVIEW2_BOOTSTRAPPER_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

/// WebView2 offline installer download URLs by architecture.
const WEBVIEW2_X64_INSTALLER_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2099617";
const WEBVIEW2_X86_INSTALLER_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2099617";
const WEBVIEW2_ARM64_INSTALLER_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2099617";

/// Download the WebView2 bootstrapper executable.
///
/// The bootstrapper is a small (~2MB) executable that downloads and installs
/// the WebView2 runtime at install time.
///
/// Returns the path to the downloaded bootstrapper exe.
pub(crate) fn download_webview2_bootstrapper(tools_dir: &Path) -> Result<PathBuf> {
    let bootstrapper_path = tools_dir.join("MicrosoftEdgeWebview2Setup.exe");

    if bootstrapper_path.exists() {
        return Ok(bootstrapper_path);
    }

    tracing::info!("Downloading WebView2 bootstrapper...");
    let data = download_bytes(WEBVIEW2_BOOTSTRAPPER_URL)?;

    std::fs::create_dir_all(tools_dir)?;
    std::fs::write(&bootstrapper_path, &data)
        .context("Failed to write WebView2 bootstrapper")?;

    Ok(bootstrapper_path)
}

/// Download the WebView2 offline installer for the given architecture.
///
/// The offline installer is a larger (~150MB) standalone installer that
/// bundles the WebView2 runtime, so no internet connection is needed at install time.
///
/// `arch` should be one of: "x64", "x86", "arm64".
///
/// Returns the path to the downloaded installer exe.
pub(crate) fn download_webview2_offline_installer(
    tools_dir: &Path,
    arch: &str,
) -> Result<PathBuf> {
    let installer_name = format!("MicrosoftEdgeWebView2RuntimeInstaller_{arch}.exe");
    let installer_path = tools_dir.join(&installer_name);

    if installer_path.exists() {
        return Ok(installer_path);
    }

    let url = match arch {
        "x64" => WEBVIEW2_X64_INSTALLER_URL,
        "x86" => WEBVIEW2_X86_INSTALLER_URL,
        "arm64" => WEBVIEW2_ARM64_INSTALLER_URL,
        _ => bail!("Unsupported architecture for WebView2 offline installer: {arch}"),
    };

    tracing::info!("Downloading WebView2 offline installer for {arch}...");
    let data = download_bytes(url)?;

    std::fs::create_dir_all(tools_dir)?;
    std::fs::write(&installer_path, &data)
        .context("Failed to write WebView2 offline installer")?;

    Ok(installer_path)
}

/// Download bytes from a URL using a blocking HTTP client.
fn download_bytes(url: &str) -> Result<Vec<u8>> {
    let response = reqwest::blocking::get(url)
        .with_context(|| format!("Failed to download {url}"))?;

    if !response.status().is_success() {
        bail!("Download failed with status {}: {url}", response.status());
    }

    response
        .bytes()
        .map(|b| b.to_vec())
        .with_context(|| format!("Failed to read response body from {url}"))
}

/// Convert a BundleContext's Arch to a Windows architecture string
/// suitable for installer file names and WebView2 downloads.
pub(crate) fn arch_to_windows_string(arch: &crate::bundler::context::Arch) -> &'static str {
    use crate::bundler::context::Arch;
    match arch {
        Arch::X86_64 => "x64",
        Arch::X86 => "x86",
        Arch::AArch64 => "arm64",
        _ => "x64", // Default to x64 for unsupported architectures
    }
}

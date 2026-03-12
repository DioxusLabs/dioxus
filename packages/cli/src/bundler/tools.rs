//! Tool downloading and caching for external bundling tools.
//!
//! All downloads happen upfront via `resolve_tools()` before any bundling starts.
//! This keeps tool downloads out of the bundle format modules.

use super::Arch;
use crate::{PackageType, WebviewInstallMode, WindowsSettings};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// NSIS download URL and expected hash
const NSIS_URL: &str =
    "https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip";
const NSIS_SHA1: &str = "EF7FF767E5CBD9EDD22ADD3A32C9B8F4500BB10D";

/// WiX download URL and expected hash
const WIX_URL: &str =
    "https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip";
const WIX_SHA256: &str = "6ac824e1642d6f7277d0ed7ea09411a508f6116ba6fae0aa5f2c7daa2ff43d31";

/// linuxdeploy download base URL
const LINUXDEPLOY_URL_BASE: &str =
    "https://github.com/tauri-apps/binary-releases/releases/download/linuxdeploy";

/// WebView2 download URLs.
const WEBVIEW2_BOOTSTRAPPER_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";
const WEBVIEW2_X64_INSTALLER_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2099617";
const WEBVIEW2_X86_INSTALLER_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2099617";
const WEBVIEW2_ARM64_INSTALLER_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2099617";

/// Pre-resolved tool paths. All downloads happen before bundling starts.
pub(crate) struct ResolvedTools {
    /// Path to the NSIS directory (contains makensis). Set if NSIS bundling is requested.
    pub nsis_dir: Option<PathBuf>,
    /// Path to the WiX directory (contains candle.exe, light.exe). Set if MSI bundling is requested.
    pub wix_dir: Option<PathBuf>,
    /// Path to the linuxdeploy binary. Set if AppImage bundling is requested.
    pub linuxdeploy: Option<PathBuf>,
    /// Path to a downloaded WebView2 bootstrapper or offline installer, if needed by NSIS.
    pub webview2_installer: Option<PathBuf>,
}

/// Resolve and download all tools needed for the given package types.
///
/// This must be called before `bundle_project()` so that no HTTP calls happen
/// during the actual bundling phase.
pub(crate) async fn resolve_tools(
    tools_dir: &Path,
    package_types: &[PackageType],
    windows_settings: &WindowsSettings,
    arch: Arch,
) -> Result<ResolvedTools> {
    let mut resolved = ResolvedTools {
        nsis_dir: None,
        wix_dir: None,
        linuxdeploy: None,
        webview2_installer: None,
    };

    for pt in package_types {
        match pt {
            PackageType::Nsis => {
                resolved.nsis_dir = Some(ensure_nsis(tools_dir).await?);
                resolved.webview2_installer =
                    resolve_webview2(tools_dir, windows_settings, arch).await?;
            }
            PackageType::WindowsMsi => {
                resolved.wix_dir = Some(ensure_wix(tools_dir).await?);
            }
            PackageType::AppImage => {
                let linuxdeploy_arch = arch.linuxdeploy_arch();
                resolved.linuxdeploy = Some(ensure_linuxdeploy(tools_dir, linuxdeploy_arch).await?);
            }
            _ => {}
        }
    }

    Ok(resolved)
}

/// Determine if a WebView2 installer needs to be downloaded based on NSIS settings,
/// and download it if so.
async fn resolve_webview2(
    tools_dir: &Path,
    settings: &WindowsSettings,
    arch: Arch,
) -> Result<Option<PathBuf>> {
    let mode = &settings.webview_install_mode;

    match mode {
        WebviewInstallMode::Skip | WebviewInstallMode::FixedRuntime { .. } => Ok(None),
        WebviewInstallMode::DownloadBootstrapper { .. }
        | WebviewInstallMode::EmbedBootstrapper { .. } => {
            Ok(Some(download_webview2_bootstrapper(tools_dir).await?))
        }
        WebviewInstallMode::OfflineInstaller { .. } => {
            let arch_str = arch.windows_arch();
            Ok(Some(
                download_webview2_offline_installer(tools_dir, arch_str).await?,
            ))
        }
    }
}

async fn ensure_nsis(tools_dir: &Path) -> Result<PathBuf> {
    let nsis_dir = tools_dir.join("nsis-3.11");
    let makensis = if cfg!(target_os = "windows") {
        nsis_dir.join("makensis.exe")
    } else {
        nsis_dir.join("makensis")
    };

    if makensis.exists() {
        return Ok(nsis_dir);
    }

    if cfg!(feature = "no-downloads") {
        bail!("NSIS not found and automatic downloads are disabled. Install NSIS manually.");
    }

    tracing::info!("Downloading NSIS...");

    let data = download_and_verify(NSIS_URL, NSIS_SHA1, HashAlgo::Sha1).await?;
    extract_zip(&data, tools_dir)?;

    if !makensis.exists() {
        bail!(
            "NSIS extraction succeeded but makensis not found at {}",
            makensis.display()
        );
    }

    #[cfg(unix)]
    let _ = std::fs::set_permissions(&makensis, std::fs::Permissions::from_mode(0o755));

    Ok(nsis_dir)
}

async fn ensure_wix(tools_dir: &Path) -> Result<PathBuf> {
    let wix_dir = tools_dir.join("wix314");
    let candle = wix_dir.join("candle.exe");

    if candle.exists() {
        return Ok(wix_dir);
    }

    if cfg!(feature = "no-downloads") {
        bail!("WiX not found and automatic downloads are disabled. Install WiX manually.");
    }

    tracing::info!("Downloading WiX toolset...");
    let data = download_and_verify(WIX_URL, WIX_SHA256, HashAlgo::Sha256).await?;

    std::fs::create_dir_all(&wix_dir)?;
    extract_zip(&data, &wix_dir)?;

    if !candle.exists() {
        bail!(
            "WiX extraction succeeded but candle.exe not found at {}",
            candle.display()
        );
    }

    Ok(wix_dir)
}

async fn ensure_linuxdeploy(tools_dir: &Path, arch: &str) -> Result<PathBuf> {
    let linuxdeploy_name = format!("linuxdeploy-{arch}.AppImage");
    let linuxdeploy_path = tools_dir.join(&linuxdeploy_name);

    if linuxdeploy_path.exists() {
        return Ok(linuxdeploy_path);
    }

    if cfg!(feature = "no-downloads") {
        bail!(
            "linuxdeploy not found and automatic downloads are disabled. Install linuxdeploy manually."
        );
    }

    let url = format!("{LINUXDEPLOY_URL_BASE}/{linuxdeploy_name}");
    tracing::info!("Downloading linuxdeploy from {url}...");

    let data = download_bytes(&url).await?;
    std::fs::create_dir_all(tools_dir)?;
    std::fs::write(&linuxdeploy_path, &data)?;

    #[cfg(unix)]
    std::fs::set_permissions(&linuxdeploy_path, std::fs::Permissions::from_mode(0o755))?;

    Ok(linuxdeploy_path)
}

async fn download_webview2_bootstrapper(tools_dir: &Path) -> Result<PathBuf> {
    let path = tools_dir.join("MicrosoftEdgeWebview2Setup.exe");
    if path.exists() {
        return Ok(path);
    }
    tracing::info!("Downloading WebView2 bootstrapper...");
    let data = download_bytes(WEBVIEW2_BOOTSTRAPPER_URL).await?;
    std::fs::create_dir_all(tools_dir)?;
    std::fs::write(&path, &data).context("Failed to write WebView2 bootstrapper")?;
    Ok(path)
}

async fn download_webview2_offline_installer(tools_dir: &Path, arch: &str) -> Result<PathBuf> {
    let name = format!("MicrosoftEdgeWebView2RuntimeInstaller_{arch}.exe");
    let path = tools_dir.join(&name);
    if path.exists() {
        return Ok(path);
    }
    let url = match arch {
        "x64" => WEBVIEW2_X64_INSTALLER_URL,
        "x86" => WEBVIEW2_X86_INSTALLER_URL,
        "arm64" => WEBVIEW2_ARM64_INSTALLER_URL,
        _ => bail!("Unsupported architecture for WebView2 offline installer: {arch}"),
    };
    tracing::info!("Downloading WebView2 offline installer for {arch}...");
    let data = download_bytes(url).await?;
    std::fs::create_dir_all(tools_dir)?;
    std::fs::write(&path, &data).context("Failed to write WebView2 offline installer")?;
    Ok(path)
}

enum HashAlgo {
    Sha1,
    Sha256,
}

async fn download_and_verify(url: &str, expected_hash: &str, algo: HashAlgo) -> Result<Vec<u8>> {
    let data = download_bytes(url).await?;

    let computed = match algo {
        HashAlgo::Sha1 => {
            use sha1::Digest;
            let mut hasher = sha1::Sha1::new();
            hasher.update(&data);
            format!("{:X}", hasher.finalize())
        }
        HashAlgo::Sha256 => {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(&data);
            format!("{:x}", hasher.finalize())
        }
    };

    if computed.to_uppercase() != expected_hash.to_uppercase() {
        bail!("Hash mismatch for {url}: expected {expected_hash}, got {computed}");
    }

    Ok(data)
}

/// Download bytes from a URL using the async reqwest client.
pub(crate) async fn download_bytes(url: &str) -> Result<Vec<u8>> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to download {url}"))?;

    if !response.status().is_success() {
        bail!("Download failed with status {}: {url}", response.status());
    }

    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .with_context(|| format!("Failed to read response body from {url}"))
}

/// Extract a zip archive to a directory.
fn extract_zip(data: &[u8], dest: &Path) -> Result<()> {
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).context("Failed to read zip archive")?;

    std::fs::create_dir_all(dest)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = dest.join(file.mangled_name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }

    Ok(())
}

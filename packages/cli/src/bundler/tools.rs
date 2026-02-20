//! Tool downloading and caching for external bundling tools.
//!
//! Downloads WiX, NSIS, and linuxdeploy to ~/.cache/dioxus/ and verifies hashes.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// NSIS download URL and expected hash
const NSIS_URL: &str =
    "https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip";
const NSIS_SHA1: &str = "EF7FF767E5CBD9EDD22ADD3A32C9B8F4500BB10D";

/// WiX download URL and expected hash
#[cfg(target_os = "windows")]
const WIX_URL: &str =
    "https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip";
#[cfg(target_os = "windows")]
const WIX_SHA256: &str = "6ac824e1642d6f7277d0ed7ea09411a508f6116ba6fae0aa5f2c7daa2ff43d31";

/// linuxdeploy download base URL
#[cfg(target_os = "linux")]
const LINUXDEPLOY_URL_BASE: &str =
    "https://github.com/tauri-apps/binary-releases/releases/download/linuxdeploy";

/// Ensure NSIS is available, downloading if necessary.
/// Returns the path to the NSIS directory containing makensis.
pub(crate) fn ensure_nsis(tools_dir: &Path) -> Result<PathBuf> {
    let nsis_dir = tools_dir.join("nsis-3.11");
    let makensis = if cfg!(target_os = "windows") {
        nsis_dir.join("makensis.exe")
    } else {
        nsis_dir.join("makensis")
    };

    if makensis.exists() {
        return Ok(nsis_dir);
    }

    #[cfg(feature = "no-downloads")]
    bail!("NSIS not found and automatic downloads are disabled. Install NSIS manually.");

    tracing::info!("Downloading NSIS...");
    let data = download_and_verify(NSIS_URL, NSIS_SHA1, HashAlgo::Sha1)?;
    extract_zip(&data, tools_dir)?;

    if !makensis.exists() {
        bail!(
            "NSIS extraction succeeded but makensis not found at {}",
            makensis.display()
        );
    }

    // Make executable on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&makensis, std::fs::Permissions::from_mode(0o755));
    }

    Ok(nsis_dir)
}

/// Ensure WiX is available, downloading if necessary.
/// Returns the path to the WiX directory containing candle.exe and light.exe.
#[cfg(target_os = "windows")]
pub(crate) fn ensure_wix(tools_dir: &Path) -> Result<PathBuf> {
    let wix_dir = tools_dir.join("wix314");
    let candle = wix_dir.join("candle.exe");

    if candle.exists() {
        return Ok(wix_dir);
    }

    #[cfg(feature = "no-downloads")]
    bail!("WiX not found and automatic downloads are disabled. Install WiX manually.");

    tracing::info!("Downloading WiX toolset...");
    let data = download_and_verify(WIX_URL, WIX_SHA256, HashAlgo::Sha256)?;

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

/// Ensure linuxdeploy is available, downloading if necessary.
/// Returns the path to the linuxdeploy binary.
#[cfg(target_os = "linux")]
pub(crate) fn ensure_linuxdeploy(tools_dir: &Path, arch: &str) -> Result<PathBuf> {
    let linuxdeploy_name = format!("linuxdeploy-{arch}.AppImage");
    let linuxdeploy_path = tools_dir.join(&linuxdeploy_name);

    if linuxdeploy_path.exists() {
        return Ok(linuxdeploy_path);
    }

    #[cfg(feature = "no-downloads")]
    bail!("linuxdeploy not found and automatic downloads are disabled. Install linuxdeploy manually.");

    let url = format!("{LINUXDEPLOY_URL_BASE}/{linuxdeploy_name}");
    tracing::info!("Downloading linuxdeploy from {url}...");

    let data = download_bytes(&url)?;
    std::fs::create_dir_all(tools_dir)?;
    std::fs::write(&linuxdeploy_path, &data)?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&linuxdeploy_path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(linuxdeploy_path)
}

enum HashAlgo {
    Sha1,
    #[cfg(target_os = "windows")]
    Sha256,
}

/// Download a URL and verify its hash.
fn download_and_verify(url: &str, expected_hash: &str, algo: HashAlgo) -> Result<Vec<u8>> {
    let data = download_bytes(url)?;

    let computed = match algo {
        HashAlgo::Sha1 => {
            use sha1::Digest;
            let mut hasher = sha1::Sha1::new();
            hasher.update(&data);
            format!("{:X}", hasher.finalize())
        }
        #[cfg(target_os = "windows")]
        HashAlgo::Sha256 => {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(&data);
            format!("{:x}", hasher.finalize())
        }
    };

    if computed.to_uppercase() != expected_hash.to_uppercase() {
        bail!(
            "Hash mismatch for {url}: expected {expected_hash}, got {computed}"
        );
    }

    Ok(data)
}

/// Download bytes from a URL using a blocking reqwest client.
fn download_bytes(url: &str) -> Result<Vec<u8>> {
    // Use a simple blocking approach - we're already in a sync context
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

            // Set permissions on unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(())
}

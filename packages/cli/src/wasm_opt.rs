use anyhow::{anyhow, Context};
use flate2::read::GzDecoder;
use tar::Archive;
use tokio::fs;

use crate::config::WasmOptLevel;
use crate::{CliSettings, Result, WasmOptConfig, Workspace};
use std::path::{Path, PathBuf};

/// Write these wasm bytes with a particular set of optimizations
pub async fn write_wasm(bytes: &[u8], output_path: &Path, cfg: &WasmOptConfig) -> Result<()> {
    std::fs::write(output_path, bytes)?;
    optimize(output_path, output_path, cfg).await?;
    Ok(())
}

pub async fn optimize(input_path: &Path, output_path: &Path, cfg: &WasmOptConfig) -> Result<()> {
    let wasm_opt_path = get_binary_path().await.unwrap();
    run_locally(&wasm_opt_path, input_path, output_path, cfg)
        .await
        .unwrap();

    Ok(())
}

async fn find_latest_wasm_opt_download_url() -> anyhow::Result<String> {
    // Find the latest release information from the GitHub API
    let url = "https://api.github.com/repos/WebAssembly/binaryen/releases/latest";
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "dioxus-cli")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    let assets = response
        .get("assets")
        .and_then(|assets| assets.as_array())
        .ok_or_else(|| anyhow::anyhow!("Failed to parse assets"))?;
    let platform = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-windows"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-linux"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-linux"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-macos"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "arm64-macos"
    } else {
        return Err(anyhow::anyhow!(
            "Unknown platform for wasm-opt installation. Please install wasm-opt manually from https://github.com/WebAssembly/binaryen/releases and add it to your PATH."
        ));
    };

    // Find the first asset with a name that contains the platform string
    let asset = assets
        .iter()
        .find(|asset| {
            asset
                .get("name")
                .and_then(|name| name.as_str())
                .map_or(false, |name| name.contains(platform))
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No suitable wasm-opt binary found for platform: {}. Please install wasm-opt manually from https://github.com/WebAssembly/binaryen/releases and add it to your PATH.",
                platform
            )
        })?;

    let download_url = asset
        .get("browser_download_url")
        .and_then(|url| url.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get download URL for wasm-opt"))?;

    Ok(download_url.to_string())
}

async fn get_binary_path() -> anyhow::Result<PathBuf> {
    let existing_path = which::which("wasm-opt");
    match existing_path {
        Ok(path) => Ok(path),
        Err(_) if CliSettings::prefer_no_downloads() => Err(anyhow!("Missing wasm-opt")),
        Err(_) => {
            let install_dir = install_dir().await?;
            let install_path = installed_bin_path(&install_dir);
            if !install_path.exists() {
                tracing::info!("Installing wasm-opt");
                install_github(&install_dir).await?;
                tracing::info!("wasm-opt installed from Github");
            }
            Ok(install_path)
        }
    }
}

async fn install_dir() -> anyhow::Result<PathBuf> {
    let bindgen_dir = Workspace::dioxus_home_dir().join("binaryen");
    fs::create_dir_all(&bindgen_dir).await?;
    Ok(bindgen_dir)
}

fn installed_bin_name() -> &'static str {
    if cfg!(windows) {
        "wasm-opt.exe"
    } else {
        "wasm-opt"
    }
}

fn installed_bin_path(install_dir: &Path) -> PathBuf {
    let bin_name = installed_bin_name();
    install_dir.join("bin").join(bin_name)
}

async fn install_github(install_dir: &Path) -> anyhow::Result<()> {
    tracing::trace!("Attempting to install wasm-opt from GitHub");

    let url = find_latest_wasm_opt_download_url().await?;
    tracing::trace!("Downloading wasm-opt from {}", url);

    // Download then extract the latest wasm-opt binary
    let bytes = reqwest::get(url).await?.bytes().await?;

    let installed_bin_path = installed_bin_path(&install_dir);
    // Unpack the dynamic library if it exists
    let lib_folder_name = "lib";
    let installed_lib_path = install_dir.join(lib_folder_name);

    // Create the lib and bin directories if they don't exist
    for path in [installed_bin_path.parent(), Some(&installed_lib_path)] {
        if let Some(path) = path {
            std::fs::create_dir_all(path)
                .context(format!("Failed to create directory: {}", path.display()))?;
        }
    }

    let mut archive = Archive::new(GzDecoder::new(bytes.as_ref()));

    // Unpack the binary and library files from the archive
    for mut entry in archive.entries()?.flatten() {
        if entry
            .path_bytes()
            .ends_with(installed_bin_name().as_bytes())
        {
            entry.unpack(&installed_bin_path)?;
        } else if let Ok(path) = entry.path() {
            if path.components().any(|c| c.as_os_str() == lib_folder_name) {
                if let Some(file_name) = path.file_name() {
                    entry.unpack(&installed_lib_path.join(file_name))?;
                }
            }
        }
    }

    Ok(())
}

async fn run_locally(
    wasm_opt_path: &Path,
    input_path: &Path,
    output_path: &Path,
    cfg: &WasmOptConfig,
) -> Result<()> {
    // defaults needed by wasm-opt.
    // wasm is a moving target, and we add these by default since they progressively get enabled by default.
    let mut args = vec![
        "--enable-reference-types",
        "--enable-bulk-memory",
        "--enable-mutable-globals",
        "--enable-nontrapping-float-to-int",
    ];

    if cfg.memory_packing {
        // needed for our current approach to bundle splitting to work properly
        // todo(jon): emit the main module's data section in chunks instead of all at once
        args.push("--memory-packing");
    }

    if !cfg.debug {
        args.push("--strip-debug");
    } else {
        args.push("--debuginfo");
    }

    for extra in &cfg.extra_features {
        args.push(extra);
    }

    let level = match cfg.level {
        WasmOptLevel::Z => "-Oz",
        WasmOptLevel::S => "-Os",
        WasmOptLevel::Zero => "-O0",
        WasmOptLevel::One => "-O1",
        WasmOptLevel::Two => "-O2",
        WasmOptLevel::Three => "-O3",
        WasmOptLevel::Four => "-O4",
    };

    let res = tokio::process::Command::new(wasm_opt_path)
        .arg(input_path)
        .arg(level)
        .arg("-o")
        .arg(output_path)
        .args(args)
        .output()
        .await?;

    if !res.status.success() {
        let err = String::from_utf8_lossy(&res.stderr);
        tracing::error!("wasm-opt failed with status code {}: {}", res.status, err);
    }

    Ok(())
}

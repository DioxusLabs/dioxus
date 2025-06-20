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
    let wasm_opt = WasmOpt::new(input_path, output_path, cfg).await?;
    wasm_opt.optimize().await?;

    Ok(())
}

struct WasmOpt {
    path: PathBuf,
    input_path: PathBuf,
    output_path: PathBuf,
    cfg: WasmOptConfig,
}

impl WasmOpt {
    pub async fn new(
        input_path: &Path,
        output_path: &Path,
        cfg: &WasmOptConfig,
    ) -> anyhow::Result<Self> {
        let path = get_binary_path().await?;
        Ok(Self {
            path,
            input_path: input_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
            cfg: cfg.clone(),
        })
    }

    /// Create the command to run wasm-opt
    fn build_command(&self) -> tokio::process::Command {
        // defaults needed by wasm-opt.
        // wasm is a moving target, and we add these by default since they progressively get enabled by default.
        let mut args = vec![
            "--enable-reference-types",
            "--enable-bulk-memory",
            "--enable-mutable-globals",
            "--enable-nontrapping-float-to-int",
        ];

        if self.cfg.memory_packing {
            // needed for our current approach to bundle splitting to work properly
            // todo(jon): emit the main module's data section in chunks instead of all at once
            args.push("--memory-packing");
        }

        if !self.cfg.debug {
            args.push("--strip-debug");
        } else {
            args.push("--debuginfo");
        }

        for extra in &self.cfg.extra_features {
            args.push(extra);
        }

        let level = match self.cfg.level {
            WasmOptLevel::Z => "-Oz",
            WasmOptLevel::S => "-Os",
            WasmOptLevel::Zero => "-O0",
            WasmOptLevel::One => "-O1",
            WasmOptLevel::Two => "-O2",
            WasmOptLevel::Three => "-O3",
            WasmOptLevel::Four => "-O4",
        };

        let mut command = tokio::process::Command::new(&self.path);
        command
            .arg(&self.input_path)
            .arg(level)
            .arg("-o")
            .arg(&self.output_path)
            .args(args);
        command
    }

    pub async fn optimize(&self) -> Result<()> {
        let mut command = self.build_command();
        let res = command.output().await?;

        if !res.status.success() {
            let err = String::from_utf8_lossy(&res.stderr);
            tracing::error!("wasm-opt failed with status code {}: {}", res.status, err);
        }

        Ok(())
    }
}

// Find the URL for the latest binaryen release that contains wasm-opt
async fn find_latest_wasm_opt_download_url() -> anyhow::Result<String> {
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

    // Find the platform identifier based on the current OS and architecture
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
                .is_some_and(|name| name.contains(platform))
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No suitable wasm-opt binary found for platform: {}. Please install wasm-opt manually from https://github.com/WebAssembly/binaryen/releases and add it to your PATH.",
                platform
            )
        })?;

    // Extract the download URL from the asset
    let download_url = asset
        .get("browser_download_url")
        .and_then(|url| url.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get download URL for wasm-opt"))?;

    Ok(download_url.to_string())
}

/// Get the path to the wasm-opt binary, downloading it if necessary
async fn get_binary_path() -> anyhow::Result<PathBuf> {
    let existing_path = which::which("wasm-opt");

    match existing_path {
        // If wasm-opt is already in the PATH, return its path
        Ok(path) => Ok(path),
        // If wasm-opt is not found in the path and we prefer no downloads, return an error
        Err(_) if CliSettings::prefer_no_downloads() => Err(anyhow!("Missing wasm-opt")),
        // Otherwise, try to install it
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

/// Install wasm-opt from GitHub releases into the specified directory
async fn install_github(install_dir: &Path) -> anyhow::Result<()> {
    tracing::trace!("Attempting to install wasm-opt from GitHub");

    let url = find_latest_wasm_opt_download_url()
        .await
        .context("Failed to find latest wasm-opt download URL")?;
    tracing::trace!("Downloading wasm-opt from {}", url);

    // Download the binaryen release archive into memory
    let bytes = reqwest::get(url).await?.bytes().await?;

    // We don't need the whole gzip archive, just the wasm-opt binary and the lib folder. We
    // just extract those files from the archive.
    let installed_bin_path = installed_bin_path(install_dir);
    let lib_folder_name = "lib";
    let installed_lib_path = install_dir.join(lib_folder_name);

    // Create the lib and bin directories if they don't exist
    for path in [installed_bin_path.parent(), Some(&installed_lib_path)]
        .into_iter()
        .flatten()
    {
        std::fs::create_dir_all(path)
            .context(format!("Failed to create directory: {}", path.display()))?;
    }

    let mut archive = Archive::new(GzDecoder::new(bytes.as_ref()));

    // Unpack the binary and library files from the archive
    for mut entry in archive.entries()?.flatten() {
        // Unpack the wasm-opt binary
        if entry
            .path_bytes()
            .ends_with(installed_bin_name().as_bytes())
        {
            entry.unpack(&installed_bin_path)?;
        }
        // Unpack any files in the lib folder
        else if let Ok(path) = entry.path() {
            if path.components().any(|c| c.as_os_str() == lib_folder_name) {
                if let Some(file_name) = path.file_name() {
                    entry.unpack(installed_lib_path.join(file_name))?;
                }
            }
        }
    }

    Ok(())
}

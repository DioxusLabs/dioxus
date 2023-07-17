use std::path::PathBuf;

use flate2::read::GzDecoder;
use tar::Archive;

use crate::{tools::TempStorage, Error, Result};

use super::ToolStorage;

const TOOL_NAME: &str = "wasm-bindgen";

// Windows
#[cfg(target_os = "windows")]
const INSTALL_URL: &str = "https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.87/wasm-bindgen-0.2.87-x86_64-pc-windows-msvc.tar.gz";
// MacOS
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const INSTALL_URL: &str = "https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.87/wasm-bindgen-0.2.87-aarch64-apple-darwin.tar.gz";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const INSTALL_URL: &str = "https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.87/wasm-bindgen-0.2.87-x86_64-apple-darwin.tar.gz";
// Linux
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const INSTALL_URL: &str = "https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.87/wasm-bindgen-0.2.87-aarch64-unknown-linux-gnu.tar.gz";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const INSTALL_URL: &str = "https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.87/wasm-bindgen-0.2.87-x86_64-unknown-linux-musl.tar.gz";

/// Get wasm-bindgen CLI's path.
pub fn get() -> Result<PathBuf> {
    // Check if exists
    let tool_storage = ToolStorage::get()?;
    let tool_path = tool_storage.get_tool_by_name(TOOL_NAME.to_string());

    // If exists return it
    if let Some(tool_path) = tool_path {
        return Ok(tool_path);
    }

    // Otherwise try installing it
    let tool_path = install()?;

    // Then return it
    Ok(tool_path)
}

/// Install the latest version of wasm-bindgen CLI.
fn install() -> Result<PathBuf> {
    log::info!("Installing wasm-bindgen...");

    let res = reqwest::blocking::get(INSTALL_URL)
        .map_err(|_| Error::CustomError("Failed to install wasm-bindgen".to_string()))?;

    let bytes = res
        .bytes()
        .map_err(|_| Error::CustomError("Failed to install wasm-bindgen".to_string()))?;

    let temp_storage = TempStorage::get()?;
    let path = temp_storage.path().join(TOOL_NAME);

    let tar = GzDecoder::new(bytes.as_ref());
    let mut archive = Archive::new(tar);
    archive.unpack(path)?;

    // Get inner path to executable
    #[cfg(target_os = "windows")]
    let bindgen_path = path.join(format!("{}.exe", TOOL_NAME));

    #[cfg(not(target_os = "windows"))]
    let bindgen_path = path.join(TOOL_NAME);

    let mut tool_storage = ToolStorage::get()?;
    tool_storage.install_tool(TOOL_NAME.to_string(), bindgen_path)
}

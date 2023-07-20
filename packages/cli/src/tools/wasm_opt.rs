use super::ToolStorage;
use crate::{tools::TempStorage, Error, Result};
use flate2::read::GzDecoder;
use std::{fs, path::PathBuf};
use tar::Archive;

const TOOL_NAME: &str = "wasm-opt";

// Windows
#[cfg(target_os = "windows")]
const INSTALL_URL: &str = "https://github.com/WebAssembly/binaryen/releases/download/version_114/binaryen-version_114-x86_64-windows.tar.gz";
// MacOS
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const INSTALL_URL: &str = "https://github.com/WebAssembly/binaryen/releases/download/version_114/binaryen-version_114-arm64-macos.tar.gz";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const INSTALL_URL: &str = "https://github.com/WebAssembly/binaryen/releases/download/version_114/binaryen-version_114-x86_64-macos.tar.gz";
// Linux
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const INSTALL_URL: &str = "https://github.com/WebAssembly/binaryen/releases/download/version_114/binaryen-version_114-x86_64-linux.tar.gz";

pub struct WasmOpt {
    exec_path: PathBuf,
}

impl WasmOpt {
    /// Get wasm-opt tool.
    pub fn get() -> Result<Self> {
        // Check if exists
        let tool_storage = ToolStorage::get()?;
        let tool_path = tool_storage.get_tool_by_name(TOOL_NAME.to_string());

        // If exists return it
        if let Some(tool_path) = tool_path {
            return Ok(Self {
                exec_path: tool_path,
            });
        }

        // Otherwise try installing it
        let exec_path = Self::install()?;

        // Then return it
        Ok(Self { exec_path })
    }

    pub fn run(&self, input: PathBuf, output: PathBuf) -> Result<()> {
        let input = fs::canonicalize(input)?;
        let output = fs::canonicalize(output)?;

        let cmd = subprocess::Exec::cmd(self.exec_path.clone())
            .arg(input)
            .arg("--output")
            .arg(output)
            .arg("-Oz");

        _ = cmd.join().map_err(|e| Error::BuildFailed(e.to_string()))?;

        Ok(())
    }

    /// Install the latest version of wasm-opt CLI.
    fn install() -> Result<PathBuf> {
        let res = reqwest::blocking::get(INSTALL_URL)
            .map_err(|_| Error::CustomError("Failed to install wasm-opt".to_string()))?;

        let bytes = res
            .bytes()
            .map_err(|_| Error::CustomError("Failed to install wasm-opt".to_string()))?;

        let temp_storage = TempStorage::get()?;
        let path = temp_storage.path().join(TOOL_NAME);

        let tar = GzDecoder::new(bytes.as_ref());
        let mut archive = Archive::new(tar);
        archive.unpack(&path)?;

        // Get inner path to exec folder. TODO: Make this 'better'
        let binding = fs::read_dir(&path)?.nth(0).unwrap()?.file_name();
        let dir_name = binding.to_str().unwrap();
        let path = path.join(dir_name).join("bin");

        // Get inner-inner path to executable
        #[cfg(target_os = "windows")]
        let bindgen_path = path.join(format!("{}.exe", TOOL_NAME));

        #[cfg(not(target_os = "windows"))]
        let bindgen_path = path.join(TOOL_NAME);

        let mut tool_storage = ToolStorage::get()?;
        tool_storage.install_tool(TOOL_NAME.to_string(), bindgen_path)
    }
}

use super::ToolStorage;
use crate::{tools::TempStorage, Error, Result};
use flate2::read::GzDecoder;
use std::{fs, path::PathBuf};
use tar::Archive;

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
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const INSTALL_URL: &str = "https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.87/wasm-bindgen-0.2.87-x86_64-unknown-linux-musl.tar.gz";

pub struct Bindgen {
    exec_path: PathBuf,
    debug: bool,
    keep_debug: bool,
    no_demangle: bool,
}

impl Bindgen {
    /// Get wasm-bindgen tool.
    pub fn get() -> Result<Self> {
        // Check if exists
        let tool_storage = ToolStorage::get()?;
        let tool_path = tool_storage.get_tool_by_name(TOOL_NAME.to_string());

        // If exists return it
        if let Some(tool_path) = tool_path {
            return Ok(Self {
                exec_path: tool_path,
                debug: true,
                keep_debug: true,
                no_demangle: true,
            });
        }

        // Otherwise try installing it
        let exec_path = Self::install()?;

        // Then return it
        Ok(Self {
            exec_path,
            debug: true,
            keep_debug: true,
            no_demangle: true,
        })
    }

    pub fn run(&self, input: PathBuf, out: PathBuf) -> Result<()> {
        if !out.exists() {
            fs::create_dir_all(&out)?;
        }

        let input = fs::canonicalize(input)?;
        let out = fs::canonicalize(out)?;

        let mut cmd = subprocess::Exec::cmd(self.exec_path.clone())
            .arg("--no-typescript")
            .arg("--target")
            .arg("web")
            .arg("--out-dir")
            .arg(out);

        if self.debug {
            cmd = cmd.arg("--debug");
        }

        if self.keep_debug {
            cmd = cmd.arg("--keep-debug");
        }

        if self.no_demangle {
            cmd = cmd.arg("--no-demangle");
        }

        _ = cmd
            .arg(input)
            .join()
            .map_err(|e| Error::BuildFailed(e.to_string()))?;

        Ok(())
    }

    pub fn debug(mut self, value: bool) -> Self {
        self.debug = value;
        self
    }

    pub fn keep_debug(mut self, value: bool) -> Self {
        self.keep_debug = value;
        self
    }

    pub fn no_demangle(mut self, value: bool) -> Self {
        self.no_demangle = value;
        self
    }

    /// Install the latest version of wasm-bindgen CLI.
    fn install() -> Result<PathBuf> {
        let res = reqwest::blocking::get(INSTALL_URL)
            .map_err(|_| Error::CustomError("Failed to install wasm-bindgen".to_string()))?;

        let bytes = res
            .bytes()
            .map_err(|_| Error::CustomError("Failed to install wasm-bindgen".to_string()))?;

        let temp_storage = TempStorage::get()?;
        let path = temp_storage.path().join(TOOL_NAME);

        let tar = GzDecoder::new(bytes.as_ref());
        let mut archive = Archive::new(tar);
        archive.unpack(&path)?;

        // Get inner path to exec folder. TODO: Make this 'better'
        let binding = fs::read_dir(&path)?.nth(0).unwrap()?.file_name();
        let dir_name = binding.to_str().unwrap();
        let path = path.join(dir_name);

        // Get inner-inner path to executable
        #[cfg(target_os = "windows")]
        let bindgen_path = path.join(format!("{}.exe", TOOL_NAME));

        #[cfg(not(target_os = "windows"))]
        let bindgen_path = path.join(TOOL_NAME);

        let mut tool_storage = ToolStorage::get()?;
        tool_storage.install_tool(TOOL_NAME.to_string(), bindgen_path)
    }
}

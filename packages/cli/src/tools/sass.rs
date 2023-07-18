use flate2::read::GzDecoder;
use tar::Archive;

use crate::{tools::TempStorage, Error, Result};
use std::{fs, path::PathBuf};

use super::ToolStorage;

const TOOL_NAME: &str = "dart-sass";

#[cfg(target_os = "windows")]
const EXEC_NAME: &str = "sass.bat";

#[cfg(not(target_os = "windows"))]
const EXEC_NAME: &str = "sass";

// Windows
#[cfg(target_os = "windows")]
const INSTALL_URL: &str =
    "https://github.com/sass/dart-sass/releases/download/1.63.6/dart-sass-1.63.6-windows-x64.zip";
// MacOS
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const INSTALL_URL: &str = "https://github.com/sass/dart-sass/releases/download/1.63.6/dart-sass-1.63.6-macos-arm64.tar.gz";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const INSTALL_URL: &str =
    "https://github.com/sass/dart-sass/releases/download/1.63.6/dart-sass-1.63.6-macos-x64.tar.gz";
// Linux
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const INSTALL_URL: &str = "https://github.com/sass/dart-sass/releases/download/1.63.6/dart-sass-1.63.6-linux-arm64.tar.gz";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const INSTALL_URL: &str =
    "https://github.com/sass/dart-sass/releases/download/1.63.6/dart-sass-1.63.6-linux-x64.tar.gz";

pub struct Sass {
    exec_path: PathBuf,
    source_map: bool,
    embed_sources: bool,
    embed_source_map: bool,
}

impl Sass {
    /// Get dart-sass tool.
    pub fn get() -> Result<Self> {
        // Check if exists
        let tool_storage = ToolStorage::get()?;
        let tool_path = tool_storage.get_tool_by_name(TOOL_NAME.to_string());

        // If exists return it
        if let Some(tool_path) = tool_path {
            return Ok(Self {
                exec_path: tool_path.join(EXEC_NAME),
                source_map: false,
                embed_sources: false,
                embed_source_map: false,
            });
        }

        // Otherwise try installing it
        let dir_path = Self::install()?;

        // Then return it
        Ok(Self {
            exec_path: dir_path.join(EXEC_NAME),
            source_map: false,
            embed_sources: false,
            embed_source_map: false,
        })
    }

    pub fn run(self, input: PathBuf, output: PathBuf) -> Result<()> {
        if !output.exists() {
            fs::create_dir_all(&output)?;
        }

        let input = fs::canonicalize(input)?;
        let output = fs::canonicalize(output)?;

        let mut cmd = subprocess::Exec::cmd(self.exec_path).arg(input).arg(output);

        if self.source_map {
            cmd = cmd.arg("--source-map");
        } else {
            cmd = cmd.arg("--no-source-map");
        }

        if self.embed_sources {
            cmd = cmd.arg("--embed-sources");
        } else {
            cmd = cmd.arg("--no-embed-sources");
        }

        if self.embed_source_map {
            cmd = cmd.arg("--embed-source-map");
        } else {
            cmd = cmd.arg("--no-embed-source-map");
        }

        _ = cmd.join().map_err(|e| Error::BuildFailed(e.to_string()))?;

        Ok(())
    }

    pub fn source_map(mut self, value: bool) -> Self {
        self.source_map = value;
        self
    }

    pub fn embed_sources(mut self, value: bool) -> Self {
        self.embed_sources = value;
        self
    }

    pub fn embed_source_map(mut self, value: bool) -> Self {
        self.embed_source_map = value;
        self
    }

    /// Install the latest version of dart-sass CLI.
    fn install() -> Result<PathBuf> {
        log::info!("Installing dart-sass...");

        let res = reqwest::blocking::get(INSTALL_URL)
            .map_err(|_| Error::CustomError("Failed to install dart-sass".to_string()))?;

        let bytes = res
            .bytes()
            .map_err(|_| Error::CustomError("Failed to install dart-sass".to_string()))?;

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
        let bindgen_path = path.join(format!("{}.bat", TOOL_NAME));

        #[cfg(not(target_os = "windows"))]
        let bindgen_path = path.join(TOOL_NAME);

        let mut tool_storage = ToolStorage::get()?;
        tool_storage.install_tool(TOOL_NAME.to_string(), bindgen_path)
    }
}

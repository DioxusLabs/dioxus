use flate2::read::GzDecoder;
use tar::Archive;

use crate::{tools::TempStorage, Error, Result};
use std::{fs, io::Cursor, path::PathBuf};

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
            });
        }

        // Otherwise try installing it
        let dir_path = Self::install()?;

        // Then return it
        Ok(Self {
            exec_path: dir_path.join(EXEC_NAME),
            source_map: false,
        })
    }

    pub fn run(&self, input: PathBuf, output: PathBuf) -> Result<()> {
        let mut cmd = subprocess::Exec::cmd(self.exec_path.clone());

        if self.source_map {
            cmd = cmd.arg("--source-map");
        } else {
            cmd = cmd.arg("--no-source-map");
        }

        _ = cmd
            .arg(input)
            .arg(output)
            .join()
            .map_err(|e| Error::BuildFailed(e.to_string()))?;

        Ok(())
    }

    pub fn source_map(mut self, value: bool) -> Self {
        self.source_map = value;
        self
    }

    /// Install the latest version of dart-sass CLI.
    fn install() -> Result<PathBuf> {
        log::info!("Installing dart-sass...");

        // Download
        let res = reqwest::blocking::get(INSTALL_URL)
            .map_err(|_| Error::CustomError("Failed to install dart-sass".to_string()))?;

        let bytes = res
            .bytes()
            .map_err(|_| Error::CustomError("Failed to install dart-sass".to_string()))?;

        let temp_storage = TempStorage::get()?;
        let path = temp_storage.path().join(TOOL_NAME);

        // If the install is for windows, the content is zipped.
        // Otherwise the content is tar.gz
        if cfg!(target_os = "windows") {
            let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
                .map_err(|e| Error::ParseError(e.to_string()))?;

            zip.extract(&path)
                .map_err(|e| Error::ParseError(e.to_string()))?
        } else {
            let tar = GzDecoder::new(bytes.as_ref());
            let mut archive = Archive::new(tar);
            archive.unpack(&path)?;
        }

        // Get path to tool folder.
        let path = path.join(TOOL_NAME);

        // Install tool
        let mut tool_storage = ToolStorage::get()?;
        tool_storage.install_tool_dir(TOOL_NAME.to_string(), path)
    }
}

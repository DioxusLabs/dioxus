use crate::{Error, Result};
use std::{ffi::OsStr, fs, path::PathBuf};

mod bindgen;
pub use bindgen::Bindgen;

const APP_DATA_NAME: &str = "dioxus";
const TEMP_NAME: &str = "temp";
const TOOLS_NAME: &str = "tools";

/// Represents the cli's data folder on the host device.
pub struct AppStorage {
    path: PathBuf,
}

impl AppStorage {
    pub fn get() -> Result<Self> {
        let data_local_path = if let Some(v) = dirs::data_local_dir() {
            v
        } else {
            return Err(Error::CustomError(
                "Failed to find your device's data directory.".to_string(),
            ));
        };

        let dioxus_dir = data_local_path.join(APP_DATA_NAME);
        if !dioxus_dir.is_dir() {
            fs::create_dir_all(&dioxus_dir).unwrap();
        }
        Ok(Self { path: dioxus_dir })
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
}

/// Represents the cli's tools folder on the host device.
pub struct ToolStorage {
    path: PathBuf,
    installed_tools: Vec<String>,
}

impl ToolStorage {
    pub fn get() -> Result<Self> {
        let app_path = AppStorage::get()?.path();
        let tools_path = app_path.join(TOOLS_NAME);
        if !tools_path.is_dir() {
            fs::create_dir_all(&tools_path).unwrap();
        }

        // Get installed tools
        let mut installed_tools = Vec::new();

        for entry in fs::read_dir(&tools_path)? {
            let entry = entry?;
            if let Some(name) = entry.path().file_stem().and_then(OsStr::to_str) {
                installed_tools.push(name.to_string());
            }
        }

        Ok(Self {
            path: tools_path,
            installed_tools,
        })
    }

    /// Get a tool by it's name.
    pub fn get_tool_by_name(&self, tool_name: String) -> Option<PathBuf> {
        if !self.is_installed(tool_name.clone()) {
            return None;
        }

        let tool_path = self.path.join(tool_name);
        Some(tool_path)
    }

    /// Check if a tool is installed.
    pub fn is_installed(&self, tool_name: String) -> bool {
        self.installed_tools.contains(&tool_name)
    }

    /// Install a new tool, replacing it if it exists.
    pub fn install_tool(&mut self, tool_name: String, tool_path: PathBuf) -> Result<PathBuf> {
        // Delete installed tool
        if self.is_installed(tool_name.clone()) {
            self.delete_tool(tool_name.clone())?;
        }

        // Copy new tool
        let full_name = format!(
            "{}.{}",
            tool_name,
            tool_path.extension().unwrap().to_str().unwrap()
        );
        let new_tool_path = self.path.join(full_name);
        fs_extra::file::copy(
            &tool_path,
            &new_tool_path,
            &fs_extra::file::CopyOptions::new()
                .overwrite(true)
                .skip_exist(false),
        )
        .map_err(|e| {
            Error::CustomError(format!(
                "Failed to replace tool `{}` from path `{}` | {} ",
                tool_name,
                tool_path.display(),
                e.to_string(),
            ))
        })?;

        Ok(new_tool_path)
    }

    /// Delete a tool if it exists.
    pub fn delete_tool(&mut self, tool_name: String) -> Result<()> {
        let path = self.path.join(tool_name.clone());
        if !path.exists() {
            return Err(Error::CustomError(format!(
                "Tool `{}` doesn't exist and can't be deleted.",
                tool_name.clone()
            )));
        }

        fs::remove_file(path)?;
        self.installed_tools.retain(|x| *x != tool_name);

        Ok(())
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
}

/// Represents the cli's temporary folder on the hot device.
pub struct TempStorage {
    path: PathBuf,
}

impl TempStorage {
    pub fn get() -> Result<Self> {
        let app_path = AppStorage::get()?.path();
        let temp_path = app_path.join(TEMP_NAME);
        if !temp_path.is_dir() {
            fs::create_dir_all(&temp_path).unwrap();
        }
        Ok(Self { path: temp_path })
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn done(&self) {
        if fs::remove_dir_all(&self.path).is_err() {
            log::warn!("Failed to delete temp directory after use.");
        }
    }
}

impl Drop for TempStorage {
    fn drop(&mut self) {
        self.done();
    }
}

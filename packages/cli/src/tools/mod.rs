use crate::{Error, Result};
use std::{fs, path::PathBuf};

pub mod bindgen;

const APP_DATA_NAME: &str = "dioxus";
const TEMP_NAME: &str = "temp";

/// Represents teh app's data folder on the host device.
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

/// Represents the temporary storage in the dioxus data folder.
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
        if fs::remove_dir_all(self.path).is_err() {
            log::warn!("Failed to delete temp directory after use.");
        }
    }
}

impl Drop for TempStorage {
    fn drop(&mut self) {
        self.done();
    }
}

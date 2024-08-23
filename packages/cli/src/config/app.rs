use super::Platform;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_platform")]
    pub default_platform: Platform,

    #[serde(default = "out_dir_default")]
    pub out_dir: PathBuf,

    #[serde(default = "asset_dir_default")]
    pub asset_dir: PathBuf,

    #[serde(default)]
    pub sub_package: Option<String>,
}

pub fn default_name() -> String {
    "my-cool-project".into()
}

pub fn default_platform() -> Platform {
    Platform::Web
}

pub fn asset_dir_default() -> PathBuf {
    PathBuf::from("public")
}

pub fn out_dir_default() -> PathBuf {
    PathBuf::from("dist")
}

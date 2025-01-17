use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ApplicationConfig {
    #[serde(default)]
    pub(crate) asset_dir: Option<PathBuf>,

    #[serde(default)]
    pub(crate) sub_package: Option<String>,

    #[serde(default)]
    pub(crate) out_dir: Option<PathBuf>,
}

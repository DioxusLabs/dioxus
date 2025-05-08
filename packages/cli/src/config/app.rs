use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ApplicationConfig {
    pub(crate) asset_dir: Option<PathBuf>,

    #[serde(default)]
    pub(crate) sub_package: Option<String>,

    #[serde(default)]
    pub(crate) out_dir: Option<PathBuf>,

    #[serde(default)]
    pub(crate) tailwind_input: Option<PathBuf>,

    #[serde(default)]
    pub(crate) tailwind_output: Option<PathBuf>,
}

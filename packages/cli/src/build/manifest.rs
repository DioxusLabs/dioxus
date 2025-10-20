//! The build manifest for `dx` applications, containing metadata about the build including
//! the CLI version, Rust version, and all bundled assets.
//!
//! We eventually plan to use this manifest to support tighter integration with deployment platforms
//! and CDNs.
//!
//! This manifest contains the list of assets, rust version, and cli version used to build the app.
//! Eventually, we might want to expand this to include more metadata about the build, including
//! build time, target platform, etc.

use std::path::PathBuf;

use dioxus_cli_opt::AssetManifest;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct AppManifest {
    pub cli_version: String,
    pub rust_version: String,
    pub assets: AssetManifest,
    pub public_items: Vec<PathBuf>,
}

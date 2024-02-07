use std::{fs::File, io::Write, path::PathBuf};

use crate::Result;
use dioxus_cli_config::CrateConfig;
use manganis_cli_support::{AssetManifest, AssetManifestExt};

pub fn asset_manifest(crate_config: &CrateConfig) -> AssetManifest {
    AssetManifest::load_from_path(
        crate_config.crate_dir.join("Cargo.toml"),
        crate_config.workspace_dir.join("Cargo.lock"),
    )
}

/// Create a head file that contains all of the imports for assets that the user project uses
pub fn create_assets_head(config: &CrateConfig, manifest: &AssetManifest) -> Result<()> {
    let mut file = File::create(config.out_dir().join("__assets_head.html"))?;
    file.write_all(manifest.head().as_bytes())?;
    Ok(())
}

/// Process any assets collected from the binary
pub(crate) fn process_assets(config: &CrateConfig, manifest: &AssetManifest) -> anyhow::Result<()> {
    let static_asset_output_dir = PathBuf::from(
        config
            .dioxus_config
            .web
            .app
            .base_path
            .clone()
            .unwrap_or_default(),
    );
    let static_asset_output_dir = config.out_dir().join(static_asset_output_dir);

    manifest.copy_static_assets_to(static_asset_output_dir)?;

    Ok(())
}

/// A guard that sets up the environment for the web renderer to compile in. This guard sets the location that assets will be served from
pub(crate) struct AssetConfigDropGuard;

impl AssetConfigDropGuard {
    pub fn new() -> Self {
        // Set up the collect asset config
        manganis_cli_support::Config::default()
            .with_assets_serve_location("/")
            .save();
        Self {}
    }
}

impl Drop for AssetConfigDropGuard {
    fn drop(&mut self) {
        // Reset the config
        manganis_cli_support::Config::default().save();
    }
}

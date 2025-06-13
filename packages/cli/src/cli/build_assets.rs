use std::{fs::create_dir_all, path::PathBuf};

use crate::{build::extract_assets_from_file, build::ejected_assets::EjectedAssets, Result, StructuredOutput};
use clap::Parser;
use dioxus_cli_opt::process_file_to;
use tracing::debug;

#[derive(Clone, Debug, Parser)]
pub struct BuildAssets {
    /// The source executable to build assets for.
    pub(crate) executable: PathBuf,

    /// The destination directory for the assets.
    pub(crate) destination: PathBuf,
}

impl BuildAssets {
    pub async fn run(&self) -> Result<StructuredOutput> {
        // Extract assets from the executable
        let manifest = extract_assets_from_file(&self.executable)?;

        // Create the output directory if it doesn't exist
        create_dir_all(&self.destination)?;
        
        // Check for ejected assets
        let ejected_assets = EjectedAssets::new();

        for asset in manifest.assets() {
            let mut source_path = PathBuf::from(asset.absolute_source_path());
            let destination_path = self.destination.join(asset.bundled_path());
            
            // Check if this asset has been ejected
            if let Some(ejected_path) = ejected_assets.get_ejected_path(asset.bundled_path()) {
                if ejected_path.exists() {
                    // Use the ejected asset instead
                    source_path = ejected_path;
                    debug!("Using ejected asset: {}", source_path.display());
                }
            }
            
            if let Some(parent) = destination_path.parent() {
                create_dir_all(parent)?;
            }
            
            debug!(
                "Processing asset {} --> {} {:#?}",
                source_path.display(),
                destination_path.display(),
                asset
            );
            
            process_file_to(asset.options(), &source_path, &destination_path)?;
        }

        Ok(StructuredOutput::Success)
    }
}

use std::{fs::create_dir_all, path::PathBuf};

use crate::{extract_assets_from_file, Result, StructuredOutput};
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
    pub async fn run(self) -> Result<StructuredOutput> {
        let manifest = extract_assets_from_file(&self.executable).await?;

        create_dir_all(&self.destination)?;
        for asset in manifest.unique_assets() {
            let source_path = PathBuf::from(asset.absolute_source_path());
            let destination_path = self.destination.join(asset.bundled_path());
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

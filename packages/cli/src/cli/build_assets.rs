use std::{fs::create_dir_all, path::PathBuf};

use crate::{Result, StructuredOutput};
use clap::Parser;
use dioxus_cli_opt::{process_file_to, AssetManifest};
use tracing::debug;

#[derive(Clone, Debug, Parser)]
pub struct BuildAssets {
    /// The source executable to build assets for.
    pub(crate) executable: PathBuf,

    /// The source directory for the assets.
    pub(crate) source: PathBuf,

    /// The destination directory for the assets.
    pub(crate) destination: PathBuf,
}

impl BuildAssets {
    pub async fn run(self) -> Result<StructuredOutput> {
        let mut manifest = AssetManifest::default();
        manifest.add_from_object_path(&self.executable)?;

        create_dir_all(&self.destination)?;
        for (path, asset) in manifest.assets.iter() {
            let source_path = self.source.join(path);
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

use super::BuildRequest;
use super::TargetPlatform;
use crate::builder::{progress::UpdateStage, MessageSource};
use crate::config::Platform;
use crate::Result;
use crate::{
    assets::OptimizeOptions,
    builder::{progress::UpdateBuildProgress, BuildMessage, MessageType},
};
use crate::{
    assets::{copy_dir_to, AssetManifest},
    link::LINK_OUTPUT_ENV_VAR,
};
use crate::{builder::progress::Stage, link::InterceptedArgs};
use anyhow::Context;
use core::str;
use futures_channel::mpsc::UnboundedSender;
use manganis_core::ResourceAsset;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    env::current_exe,
    fs::{self, create_dir_all},
    io::Read,
    sync::{atomic::AtomicUsize, Arc},
};
use std::{
    io::{BufWriter, Write},
    path::Path,
};
use std::{path::PathBuf, process::Stdio};
use tokio::process::Command;
use tracing::Level;

impl BuildRequest {
    /// Run the linker intercept and then fill in our AssetManifest from the incremental artifacts
    ///
    /// This will execute `dx` with an env var set to force `dx` to operate as a linker, and then
    /// traverse the .o and .rlib files rustc passes that new `dx` instance, collecting the link
    /// tables marked by manganis and parsing them as a ResourceAsset.
    pub async fn collect_assets(&mut self, cargo_args: Vec<String>) -> anyhow::Result<()> {
        // If this is the server build, the client build already copied any assets we need
        if self.target_platform == TargetPlatform::Server {
            return Ok(());
        }

        // If assets are skipped, we don't need to collect them
        if self.build_arguments.skip_assets {
            return Ok(());
        }

        // Create a temp file to put the output of the args
        // We need to do this since rustc won't actually print the link args to stdout, so we need to
        // give `dx` a file to dump its env::args into
        let tmp_file = tempfile::NamedTempFile::new()?;

        // Run `cargo rustc` again, but this time with a custom linker (dx) and an env var to force
        // `dx` to act as a linker
        //
        // Pass in the tmp_file as the env var itself
        //
        // NOTE: that -Csave-temps=y is needed to prevent rustc from deleting the incremental cache...
        // This might not be a "stable" way of keeping artifacts around, but it's in stable rustc
        tokio::process::Command::new("cargo")
            .arg("rustc")
            .args(cargo_args)
            .arg("--offline") /* don't use the network, should already be resolved */
            .arg("--")
            .arg(format!("-Clinker={}", current_exe().unwrap().display())) /* pass ourselves in */
            .env(LINK_OUTPUT_ENV_VAR, tmp_file.path()) /* but with the env var pointing to the temp file */
            .arg("-Csave-temps=y") /* don't delete the incremental cache */
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Read the contents of the temp file
        let args = std::fs::read_to_string(tmp_file.path()).expect("Failed to read linker output");

        // Parse them as a Vec<String> which is just our informal format for link args in the cli
        // Todo: this might be wrong-ish on windows? The format is weird
        let args =
            serde_json::from_str::<InterceptedArgs>(&args).expect("Failed to parse linker output");

        self.assets.add_from_linker_intercept(args);

        Ok(())
    }

    pub fn copy_assets_dir(&self) -> anyhow::Result<()> {
        tracing::info!("Copying public assets to the output directory...");

        let static_asset_output_dir = self.target_out_dir();
        std::fs::create_dir_all(&static_asset_output_dir)
            .context("Failed to create static asset output directory")?;

        // todo: join the entire asset dir here
        let asset_dir = self.krate.asset_dir();
        let assets = self.assets.assets.keys().collect::<Vec<_>>();

        let assets_finished = AtomicUsize::new(0);
        let asset_count = assets.len();
        let manifest = &self.assets;
        let platform = self.target_platform;

        let options = OptimizeOptions {
            precompress: self.should_precompress_assets(),
            enabled: false,
        };

        assets
            .par_iter()
            .enumerate()
            .try_for_each(|(_idx, asset)| {
                // Update the progress
                _ = self.progress.unbounded_send(UpdateBuildProgress {
                    stage: Stage::OptimizingAssets,
                    update: UpdateStage::AddMessage(BuildMessage {
                        level: Level::INFO,
                        message: MessageType::Text(format!(
                            "Optimized static asset {}",
                            asset.display()
                        )),
                        source: MessageSource::Build,
                    }),
                    platform,
                });

                // Copy the asset into the bundled d
                manifest.copy_asset_to(
                    static_asset_output_dir.clone(),
                    asset.to_path_buf(),
                    &options,
                );

                let finished = assets_finished.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                _ = self.progress.unbounded_send(UpdateBuildProgress {
                    stage: Stage::OptimizingAssets,
                    update: UpdateStage::SetProgress(finished as f64 / asset_count as f64),
                    platform,
                });

                Ok(()) as anyhow::Result<()>
            })?;

        Ok(())
    }

    fn should_precompress_assets(&self) -> bool {
        self.targeting_web()
            && self
                .krate
                .should_pre_compress_web_assets(self.build_arguments.release)
    }
}

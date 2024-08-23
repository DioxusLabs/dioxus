use super::BuildRequest;
use super::TargetPlatform;
use crate::assets::{copy_dir_to, AssetManifest};
use crate::builder::progress::CargoBuildResult;
use crate::builder::progress::Stage;
use crate::builder::progress::UpdateBuildProgress;
use crate::builder::progress::UpdateStage;
use crate::config::Platform;
use crate::link::LinkCommand;
use crate::Result;
use anyhow::Context;
use futures_channel::mpsc::UnboundedSender;
use std::fs::create_dir_all;
use std::path::PathBuf;
use tokio::process::Command;

type ProgressChannel = UnboundedSender<UpdateBuildProgress>;

impl BuildRequest {
    pub async fn collect_assets(
        &mut self,
        cargo_args: Vec<String>,
    ) -> anyhow::Result<Option<AssetManifest>> {
        // If this is the server build, the client build already copied any assets we need
        if self.target_platform == TargetPlatform::Server {
            return Ok(None);
        }

        // If assets are skipped, we don't need to collect them
        if self.build_arguments.skip_assets {
            return Ok(None);
        }

        Ok(None)

        // // Start Manganis linker intercept.
        // let linker_args = vec![format!("{}", self.target_out_dir().display())];

        // // Don't block the main thread - manganis should not be running its own std process but it's
        // // fine to wrap it here at the top
        // let build = self.clone();
        // let mut progress = progress.clone();
        // tokio::task::spawn_blocking(move || {
        //     manganis_cli_support::start_linker_intercept(
        //         &LinkCommand::command_name(),
        //         cargo_args,
        //         Some(linker_args),
        //     )?;
        //     let assets = asset_manifest(&build);
        //     // Collect assets from the asset manifest the linker intercept created
        //     process_assets(&build, &assets, &mut progress)?;
        //     // Create the __assets_head.html file for bundling
        //     create_assets_head(&build, &assets)?;

        //     Ok(Some(assets))
        // })
        // .await
        // .unwrap()
    }

    pub fn copy_assets_dir(&self) -> anyhow::Result<()> {
        tracing::info!("Copying public assets to the output directory...");
        let out_dir = self.target_out_dir();
        let asset_dir = self.krate.asset_dir();

        if asset_dir.is_dir() {
            // Only pre-compress the assets from the web build. Desktop assets are not served, so they don't need to be pre_compressed
            let pre_compress = self.targeting_web()
                && self
                    .krate
                    .should_pre_compress_web_assets(self.build_arguments.release);

            copy_dir_to(asset_dir, out_dir, pre_compress)?;
        }

        Ok(())
    }
}

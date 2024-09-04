use super::{BuildRequest, BuildResult};
use crate::Result;
use crate::{assets::AssetManifest, builder::progress::*};
use crate::{builder::Platform, bundler::AppBundle};
use anyhow::Context;
use std::fs::create_dir_all;
use std::path::PathBuf;
use tokio::process::Command;

impl BuildRequest {
    pub async fn build(self) -> Result<BuildResult> {
        tracing::info!("🚅 Running build [Desktop] command...");

        // Install any tooling that might be required for this build.
        self.verify_tooling().await?;

        // Run the build command with a pretty loader, returning the executable output location
        let executable = self.build_cargo().await?;

        // Extract out the asset manifest from the executable using our linker tricks
        let assets = self.collect_assets().await?;

        // Assemble a bundle from everything
        let bundle = self.bundle_app(executable, &assets).await?;

        // And then construct a final BuildResult which we can then modify while the app is running
        BuildResult::new(self, assets, bundle)
            .await
            .map_err(Into::into)
    }

    pub async fn verify_tooling(&self) -> Result<()> {
        // If this is a web, build make sure we have the web build tooling set up
        if self.targeting_web() {
            self.install_web_build_tooling().await?;
        }

        Ok(())
    }

    pub(crate) async fn bundle_app(
        &self,
        executable: PathBuf,
        assets: &AssetManifest,
    ) -> Result<AppBundle> {
        let mut bundle = AppBundle::new(self.platform());

        bundle.copy_assets(assets);

        //     _ = self.progress.unbounded_send(UpdateBuildProgress {
        //         stage: Stage::OptimizingAssets,
        //         update: UpdateStage::Start,
        //         platform: self.target_platform,
        //     });

        //     self.collect_assets().await?;

        //     let file_name = self.krate.executable_name();

        //     // Move the final output executable into the dist folder
        //     let out_dir = self.target_out_dir();
        //     if !out_dir.is_dir() {
        //         create_dir_all(&out_dir)?;
        //     }

        //     let mut output_path = out_dir.join(file_name);

        //     // todo: this should not be platform cfged but rather be a target config
        //     // we dont always want to set the .exe extension...
        //     if self.targeting_web() {
        //         output_path.set_extension("wasm");
        //     } else if cfg!(windows) {
        //         output_path.set_extension("exe");
        //     }

        //     // if let Some(res_path) = &cargo_build_result.output_location {
        //     //     std::fs::copy(res_path, &output_path)?;
        //     // }

        //     // // Make sure we set the exeutable
        //     // self.executable = Some(output_path.canonicalize()?);

        //     // // And then copy over the asset dir into the bundle
        //     // // todo: this will eventually become a full bundle step
        //     // self.copy_assets_dir()?;

        //     // If this is a web build, run web post processing steps
        //     if self.targeting_web() {
        //         self.post_process_web_build().await?;
        //     }

        todo!()
    }

    /// Get the output directory for a specific built target
    pub fn target_out_dir(&self) -> PathBuf {
        let out_dir = self.krate.out_dir();

        todo!()

        // if let Some(Platform::Fullstack) = self.build_arguments.platform {
        //     match self.platform {
        //         Platform::Web => out_dir.join("public"),
        //         Platform::Desktop => out_dir.join("desktop"),
        //         _ => out_dir,
        //     }
        // } else {
        //     out_dir
        // }
    }

    /// Create a list of arguments for cargo builds
    pub(crate) fn build_arguments(&self) -> Vec<String> {
        let mut cargo_args = Vec::new();

        if self.build.release {
            cargo_args.push("--release".to_string());
        }
        if self.build.verbose {
            cargo_args.push("--verbose".to_string());
        } else {
            cargo_args.push("--quiet".to_string());
        }

        if let Some(custom_profile) = &self.build.profile {
            cargo_args.push("--profile".to_string());
            cargo_args.push(custom_profile.to_string());
        }

        if !self.build.target_args.features.is_empty() {
            let features_str = self.build.target_args.features.join(" ");
            cargo_args.push("--features".to_string());
            cargo_args.push(features_str);
        }

        if let Some(target) = self
            .targeting_web()
            .then_some("wasm32-unknown-unknown")
            .or(self.build.target_args.target.as_deref())
        {
            cargo_args.push("--target".to_string());
            cargo_args.push(target.to_string());
        }

        if let Some(ref platform) = self.build.target_args.package {
            cargo_args.push(String::from("-p"));
            cargo_args.push(platform.clone());
        }

        cargo_args.append(&mut self.build.cargo_args.clone());

        match self.krate.executable_type() {
            krates::cm::TargetKind::Bin => {
                cargo_args.push("--bin".to_string());
            }
            krates::cm::TargetKind::Lib => {
                cargo_args.push("--lib".to_string());
            }
            krates::cm::TargetKind::Example => {
                cargo_args.push("--example".to_string());
            }
            _ => {}
        };

        cargo_args.push(self.krate.executable_name().to_string());

        cargo_args
    }
}

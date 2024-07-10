use super::web::install_web_build_tooling;
use super::BuildRequest;
use super::BuildResult;
use crate::assets::copy_dir_to;
use crate::assets::create_assets_head;
use crate::assets::{asset_manifest, process_assets, AssetConfigDropGuard};
use crate::builder::progress::build_cargo;
use crate::builder::progress::CargoBuildResult;
use crate::builder::progress::Stage;
use crate::builder::progress::UpdateBuildProgress;
use crate::builder::progress::UpdateStage;
use crate::link::LinkCommand;
use crate::ExecutableType;
use crate::Result;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use manganis_cli_support::ManganisSupportGuard;
use std::env;
use std::fs::create_dir_all;
use std::time::Instant;
use tokio::process::Command;

impl BuildRequest {
    /// Create a build command for cargo
    fn prepare_build_command(&self) -> Result<(tokio::process::Command, Vec<String>)> {
        let mut cargo_args = Vec::new();

        let mut cmd = tokio::process::Command::new("cargo");
        if let Some(target_dir) = &self.target_dir {
            cmd.env("CARGO_TARGET_DIR", target_dir);
        }
        cmd.current_dir(&self.config.crate_dir())
            .arg("build")
            .arg("--message-format=json-render-diagnostics");

        set_rust_flags(&mut cmd, self.rust_flags.clone());

        if self.build_arguments.release {
            cargo_args.push("--release".to_string());
        }
        if self.build_arguments.verbose {
            cargo_args.push("--verbose".to_string());
        } else {
            cargo_args.push("--quiet".to_string());
        }

        if let Some(custom_profile) = &self.build_arguments.profile {
            cargo_args.push("--profile".to_string());
            cargo_args.push(custom_profile.to_string());
        }

        if !self.build_arguments.target_args.features.is_empty() {
            let features_str = self.build_arguments.target_args.features.join(" ");
            cargo_args.push("--features".to_string());
            cargo_args.push(features_str);
        }

        if let Some(target) = self.web.then_some("wasm32-unknown-unknown").or(self
            .build_arguments
            .target_args
            .target
            .as_deref())
        {
            cargo_args.push("--target".to_string());
            cargo_args.push(target.to_string());
        }

        cargo_args.append(&mut self.build_arguments.cargo_args.clone());

        match self.config.executable_type() {
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
        cargo_args.push(self.config.executable_name().to_string());

        cmd.args(&cargo_args);

        Ok((cmd, cargo_args))
    }

    pub async fn build(
        &self,
        mut progress: UnboundedSender<UpdateBuildProgress>,
    ) -> Result<BuildResult> {
        tracing::info!("🚅 Running build [Desktop] command...");

        // Set up runtime guards
        let start_time = std::time::Instant::now();
        let _guard = dioxus_cli_config::__private::save_config(&self.config.dioxus_config);
        let _manganis_support = ManganisSupportGuard::default();
        let _asset_guard = AssetConfigDropGuard::new();

        // If this is a web, build make sure we have the web build tooling set up
        if self.web {
            install_web_build_tooling(&mut progress).await?;
        }

        // Create the build command
        let (cmd, cargo_args) = self.prepare_build_command()?;

        // Run the build command with a pretty loader
        // TODO: use https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#unit-graph once it is stable
        let crate_count: usize = self
            .config
            .krates
            .krates_filtered(krates::DepKind::Dev)
            .iter()
            .map(|k| k.targets.len())
            .sum::<usize>()
            / 4;
        let cargo_result = build_cargo(crate_count, cmd, &mut progress).await?;

        // Post process the build result
        let build_result = self
            .post_process_build(cargo_args, &cargo_result, start_time, &mut progress)
            .await?;

        tracing::info!(
            "🚩 Build completed: [./{}]",
            self.config
                .dioxus_config
                .application
                .out_dir
                .clone()
                .display()
        );

        _ = progress.start_send(UpdateBuildProgress {
            stage: Stage::Finished,
            update: UpdateStage::Start,
        });

        Ok(build_result)
    }

    async fn post_process_build(
        &self,
        cargo_args: Vec<String>,
        cargo_build_result: &CargoBuildResult,
        t_start: Instant,
        progress: &mut UnboundedSender<UpdateBuildProgress>,
    ) -> Result<BuildResult> {
        _ = progress.start_send(UpdateBuildProgress {
            stage: Stage::OptimizingAssets,
            update: UpdateStage::Start,
        });

        // Start Manganis linker intercept.
        let linker_args = vec![format!("{}", self.config.out_dir().display())];

        manganis_cli_support::start_linker_intercept(
            &LinkCommand::command_name(),
            cargo_args,
            Some(linker_args),
        )?;

        let file_name = self.config.executable_name();

        let out_dir = self.config.out_dir();
        if !out_dir.is_dir() {
            create_dir_all(&out_dir)?;
        }
        let mut output_path = out_dir.join(file_name);
        if self.web {
            output_path.set_extension("wasm");
        } else if cfg!(windows) {
            output_path.set_extension("exe");
        }
        if let Some(res_path) = &cargo_build_result.output_location {
            std::fs::copy(res_path, &output_path)?;
        }

        self.copy_assets_dir()?;

        let assets = if !self.build_arguments.skip_assets {
            let assets = asset_manifest(&self.config);
            // Collect assets
            process_assets(&self.config, &assets)?;
            // Create the __assets_head.html file for bundling
            create_assets_head(&self.config, &assets)?;
            Some(assets)
        } else {
            None
        };

        // Create the build result
        let build_result = BuildResult {
            executable: output_path,
            elapsed_time: t_start.elapsed(),
            web: self.web,
        };

        // If this is a web build, run web post processing steps
        if self.web {
            self.post_process_web_build(&build_result, assets.as_ref(), progress)
                .await?;
        }

        Ok(build_result)
    }

    pub fn copy_assets_dir(&self) -> anyhow::Result<()> {
        tracing::info!("Copying public assets to the output directory...");
        let out_dir = self.config.out_dir();
        let asset_dir = self.config.asset_dir();

        if asset_dir.is_dir() {
            // Only pre-compress the assets from the web build. Desktop assets are not served, so they don't need to be pre_compressed
            let pre_compress = self.web
                && self
                    .config
                    .should_pre_compress_web_assets(self.build_arguments.release);

            copy_dir_to(asset_dir, out_dir, pre_compress)?;
        }
        Ok(())
    }
}

/// Sets (appends to, if already set) `RUSTFLAGS` environment variable if
/// `rust_flags` is not `None`.
fn set_rust_flags(command: &mut Command, rust_flags: Option<String>) {
    if let Some(rust_flags) = rust_flags {
        // Some `RUSTFLAGS` might be already set in the environment or provided
        // by the user. They should take higher priority than the default flags.
        // If no default flags are provided, then there is no point in
        // redefining the environment variable with the same value, if it is
        // even set. If no custom flags are set, then there is no point in
        // adding the unnecessary whitespace to the command.
        command.env(
            "RUSTFLAGS",
            if let Ok(custom_rust_flags) = env::var("RUSTFLAGS") {
                rust_flags + " " + custom_rust_flags.as_str()
            } else {
                rust_flags
            },
        );
    }
}

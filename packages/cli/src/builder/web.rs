use super::{BuildRequest, TargetPlatform};
use crate::assets::pre_compress_folder;
use crate::builder::progress::Stage;
use crate::builder::progress::UpdateBuildProgress;
use crate::builder::progress::UpdateStage;
use crate::error::{Error, Result};
use std::path::Path;
use tokio::process::Command;
use wasm_bindgen_cli_support::Bindgen;

impl BuildRequest {
    async fn run_wasm_bindgen(&self, input_path: &Path, bindgen_outdir: &Path) -> Result<()> {
        tracing::info!("Running wasm-bindgen");
        let input_path = input_path.to_path_buf();
        let bindgen_outdir = bindgen_outdir.to_path_buf();
        let keep_debug =
            self.krate.dioxus_config.web.wasm_opt.debug || (!self.build_arguments.release);
        let name = self.krate.dioxus_config.application.name.clone();

        let run_wasm_bindgen = move || {
            // [3] Bindgen the final binary for use easy linking
            let mut bindgen_builder = Bindgen::new();

            bindgen_builder
                .input_path(&input_path)
                .web(true)
                .unwrap()
                .debug(keep_debug)
                .demangle(keep_debug)
                .keep_debug(keep_debug)
                .reference_types(true)
                .remove_name_section(!keep_debug)
                .remove_producers_section(!keep_debug)
                .out_name(&name)
                .generate(&bindgen_outdir)
                .unwrap();
        };

        let start = std::time::Instant::now();
        let bindgen_result = tokio::task::spawn_blocking(run_wasm_bindgen.clone()).await;

        tracing::info!("wasm-bindgen complete in {:?}", start.elapsed());

        // WASM bindgen requires the exact version of the bindgen schema to match the version the CLI was built with
        // If we get an error, we can try to recover by pinning the user's wasm-bindgen version to the version we used
        if let Err(err) = bindgen_result {
            tracing::error!("Bindgen build failed: {:?}", err);
            Self::update_wasm_bindgen_version().await?;
            run_wasm_bindgen();
        }

        Ok(())
    }

    /// Post process the WASM build artifacts
    pub(crate) async fn post_process_web_build(&mut self) -> Result<()> {
        _ = self.progress.start_send(UpdateBuildProgress {
            stage: Stage::OptimizingWasm,
            update: UpdateStage::Start,
            platform: self.target_platform,
        });

        // Find the wasm file
        let output_location = self.executable.clone().unwrap();
        let input_path = output_location.with_extension("wasm");

        // Create the directory where the bindgen output will be placed
        let bindgen_outdir = self.target_out_dir().join("assets").join("dioxus");

        // Run wasm-bindgen
        self.run_wasm_bindgen(&input_path, &bindgen_outdir).await?;

        // Only run wasm-opt if the feature is enabled
        // Wasm-opt has an expensive build script that makes it annoying to keep enabled for iterative dev
        // We put it behind the "wasm-opt" feature flag so that it can be disabled when iterating on the cli
        #[cfg(feature = "wasm-opt")]
        self.run_wasm_opt(&bindgen_outdir)?;

        // If pre-compressing is enabled, we can pre_compress the wasm-bindgen output
        let pre_compress = self
            .krate
            .should_pre_compress_web_assets(self.build_arguments.release);

        tokio::task::spawn_blocking(move || pre_compress_folder(&bindgen_outdir, pre_compress))
            .await
            .unwrap()?;

        // Create the index.html file
        // Note that we do this last since the webserver will attempt to serve the index.html file
        // If we do this too early, the wasm won't be ready but the index.html will be served, leading
        // to test failures and broken pages.
        let html = self.prepare_html()?;
        let html_path = self.target_out_dir().join("index.html");
        std::fs::write(html_path, html)?;

        Ok(())
    }

    /// Check if the wasm32-unknown-unknown target is installed and try to install it if not
    pub(crate) async fn install_web_build_tooling(&mut self) -> Result<()> {
        // If the user has rustup, we can check if the wasm32-unknown-unknown target is installed
        // Otherwise we can just assume it is installed - which is not great...
        // Eventually we can poke at the errors and let the user know they need to install the target
        if let Ok(wasm_check_command) = Command::new("rustup").args(["show"]).output().await {
            let wasm_check_output = String::from_utf8(wasm_check_command.stdout).unwrap();
            if !wasm_check_output.contains("wasm32-unknown-unknown") {
                _ = self.progress.start_send(UpdateBuildProgress {
                    stage: Stage::InstallingWasmTooling,
                    update: UpdateStage::Start,
                    platform: self.target_platform,
                });
                tracing::info!("wasm32-unknown-unknown target not detected, installing..");
                let _ = Command::new("rustup")
                    .args(["target", "add", "wasm32-unknown-unknown"])
                    .output()
                    .await?;
            }
        }

        Ok(())
    }

    // Attempt to automatically recover from a bindgen failure by updating the wasm-bindgen version
    pub(crate) async fn update_wasm_bindgen_version() -> Result<()> {
        let cli_bindgen_version = wasm_bindgen_shared::version();
        tracing::info!("Attempting to recover from bindgen failure by setting the wasm-bindgen version to {cli_bindgen_version}...");

        let output = Command::new("cargo")
            .args([
                "update",
                "-p",
                "wasm-bindgen",
                "--precise",
                &cli_bindgen_version,
            ])
            .output()
            .await;

        let mut error_message = None;
        if let Ok(output) = output {
            if output.status.success() {
                tracing::info!("Successfully updated wasm-bindgen to {cli_bindgen_version}");
                return Ok(());
            } else {
                error_message = Some(output);
            }
        }

        if let Some(output) = error_message {
            tracing::error!("Failed to update wasm-bindgen: {:#?}", output);
        }

        Err(Error::BuildFailed(format!("WASM bindgen build failed!\nThis is probably due to the Bindgen version, dioxus-cli is using `{cli_bindgen_version}` which is not compatible with your crate.\nPlease reinstall the dioxus cli to fix this issue.\nYou can reinstall the dioxus cli by running `cargo install dioxus-cli --force` and then rebuild your project")))
    }

    #[cfg(feature = "wasm-opt")]
    fn run_wasm_opt(&mut self, bindgen_outdir: &std::path::PathBuf) -> Result<(), Error> {
        if !self.build_arguments.release {
            return Ok(());
        };

        use crate::config::WasmOptLevel;

        tracing::info!("Running optimization with wasm-opt...");
        let mut options = match self.dioxus_crate.dioxus_config.web.wasm_opt.level {
            WasmOptLevel::Z => wasm_opt::OptimizationOptions::new_optimize_for_size_aggressively(),
            WasmOptLevel::S => wasm_opt::OptimizationOptions::new_optimize_for_size(),
            WasmOptLevel::Zero => wasm_opt::OptimizationOptions::new_opt_level_0(),
            WasmOptLevel::One => wasm_opt::OptimizationOptions::new_opt_level_1(),
            WasmOptLevel::Two => wasm_opt::OptimizationOptions::new_opt_level_2(),
            WasmOptLevel::Three => wasm_opt::OptimizationOptions::new_opt_level_3(),
            WasmOptLevel::Four => wasm_opt::OptimizationOptions::new_opt_level_4(),
        };
        let wasm_file = bindgen_outdir.join(format!(
            "{}_bg.wasm",
            self.dioxus_crate.dioxus_config.application.name
        ));
        let old_size = wasm_file.metadata()?.len();
        options
            // WASM bindgen relies on reference types
            .enable_feature(wasm_opt::Feature::ReferenceTypes)
            .debug_info(self.dioxus_crate.dioxus_config.web.wasm_opt.debug)
            .run(&wasm_file, &wasm_file)
            .map_err(|err| Error::Other(anyhow::anyhow!(err)))?;
        let new_size = wasm_file.metadata()?.len();
        tracing::info!(
            "wasm-opt reduced WASM size from {} to {} ({:2}%)",
            old_size,
            new_size,
            (new_size as f64 - old_size as f64) / old_size as f64 * 100.0
        );

        Ok(())
    }

    /// Check if the build is targeting the web platform
    pub fn targeting_web(&self) -> bool {
        self.target_platform == TargetPlatform::Web
    }
}

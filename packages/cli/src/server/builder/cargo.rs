use super::BuildRequest;
use super::BuildResult;
use crate::assets::{asset_manifest, copy_assets_dir, process_assets, AssetConfigDropGuard};
use crate::Result;
use dioxus_cli_config::{CrateConfig, ExecutableType, Platform};
use manganis_cli_support::{AssetManifest, ManganisSupportGuard};
use std::{env, path::PathBuf};

impl BuildRequest {
    /// Create a build command for cargo
    pub fn prepare_build_command(&self) -> Result<subprocess::Exec> {
        let mut cargo_args = Vec::new();

        let mut cmd = subprocess::Exec::cmd("cargo")
            .set_rust_flags(self.build_arguments.rust_flags)
            .env("CARGO_TARGET_DIR", &self.config.target_dir)
            .cwd(&self.config.crate_dir)
            .arg("build")
            .arg("--message-format=json-render-diagnostics");

        if self.config.release {
            cargo_args.push("--release".to_string());
        }
        if self.config.verbose {
            cargo_args.push("--verbose".to_string());
        } else {
            cargo_args.push("--quiet".to_string());
        }

        if self.config.custom_profile.is_some() {
            let custom_profile = self.config.custom_profile.as_ref().unwrap();
            cargo_args.push("--profile".to_string());
            cargo_args.push(custom_profile.to_string());
        }

        if self.config.features.is_some() {
            let features_str = self.config.features.as_ref().unwrap().join(" ");
            cargo_args.push("--features".to_string());
            cargo_args.push(features_str);
        }

        if let Some(target) = &self.config.target {
            cargo_args.push("--target".to_string());
            cargo_args.push(target.clone());
        }

        cargo_args.append(&mut self.config.cargo_args.clone());

        match &self.config.executable {
            ExecutableType::Binary(name) => {
                cargo_args.push("--bin".to_string());
                cargo_args.push(name.to_string());
            }
            ExecutableType::Lib(name) => {
                cargo_args.push("--lib".to_string());
                cargo_args.push(name.to_string());
            }
            ExecutableType::Example(name) => {
                cargo_args.push("--example".to_string());
                cargo_args.push(name.to_string());
            }
        };

        cmd = cmd.args(&cargo_args);

        Ok(cmd)
    }

    pub fn build(&self) -> Result<BuildResult> {
        tracing::info!("🚅 Running build [Desktop] command...");

        // Set up runtime guards
        let t_start = std::time::Instant::now();
        let _guard = dioxus_cli_config::__private::save_config(config);
        let _manganis_support = ManganisSupportGuard::default();
        let _asset_guard = AssetConfigDropGuard::new();

        // If this is a web, build make sure we have the web build tooling set up
        install_web_build_tooling()?;

        // Create the build command
        let cmd = self.build_command()?;

        // Run the build command with a pretty loader
        let warning_messages = build_cargo(cmd)?;

        // Post process the build result

        // Start Manganis linker intercept.
        let linker_args = vec![format!("{}", self.config.out_dir().display())];

        manganis_cli_support::start_linker_intercept(
            &LinkCommand::command_name(),
            cargo_args,
            Some(linker_args),
        )?;

        let file_name: String = self.config.executable.executable().unwrap().to_string();

        let target_file = if cfg!(windows) {
            format!("{}.exe", &file_name)
        } else {
            file_name
        };

        if !config.out_dir().is_dir() {
            create_dir_all(config.out_dir())?;
        }
        let output_path = self.config.out_dir().join(target_file);
        if let Some(res_path) = &warning_messages.output_location {
            copy(res_path, &output_path)?;
        }

        // Create the build result
        let build_result = BuildResult {
            warnings: warning_messages.warnings,
            executable: Some(output_path),
            elapsed_time: t_start.elapsed(),
            assets,
        };

        // If this is a web build, run web post processing steps
        if self.web {
            self.post_process_web_build(&build_result)
        }

        copy_assets_dir(config, self.compress_assets)?;

        let assets = if !skip_assets {
            tracing::info!("Processing assets");
            let assets = asset_manifest(config);
            // Collect assets
            process_assets(config, &assets)?;
            // Create the __assets_head.html file for bundling
            create_assets_head(config, &assets)?;
            Some(assets)
        } else {
            None
        };

        tracing::info!(
            "🚩 Build completed: [./{}]",
            self.config
                .dioxus_config
                .application
                .out_dir
                .clone()
                .display()
        );

        Ok(build_result)
    }
}
/// This trait is only created for the convenient and concise way to set
/// `RUSTFLAGS` environment variable for the `subprocess::Exec`.
pub trait ExecWithRustFlagsSetter {
    fn set_rust_flags(self, rust_flags: Option<String>) -> Self;
}

impl ExecWithRustFlagsSetter for subprocess::Exec {
    /// Sets (appends to, if already set) `RUSTFLAGS` environment variable if
    /// `rust_flags` is not `None`.
    fn set_rust_flags(self, rust_flags: Option<String>) -> Self {
        if let Some(rust_flags) = rust_flags {
            // Some `RUSTFLAGS` might be already set in the environment or provided
            // by the user. They should take higher priority than the default flags.
            // If no default flags are provided, then there is no point in
            // redefining the environment variable with the same value, if it is
            // even set. If no custom flags are set, then there is no point in
            // adding the unnecessary whitespace to the command.
            self.env(
                "RUSTFLAGS",
                if let Ok(custom_rust_flags) = env::var("RUSTFLAGS") {
                    rust_flags + " " + custom_rust_flags.as_str()
                } else {
                    rust_flags
                },
            )
        } else {
            self
        }
    }
}

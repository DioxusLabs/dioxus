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
}

/// Note: `rust_flags` argument is only used for the fullstack platform
/// (server).
pub fn build_desktop(
    config: &CrateConfig,
    _is_serve: bool,
    skip_assets: bool,
    rust_flags: Option<String>,
) -> Result<BuildResult> {
    tracing::info!("🚅 Running build [Desktop] command...");

    let t_start = std::time::Instant::now();
    let _guard = dioxus_cli_config::__private::save_config(config);
    let _manganis_support = ManganisSupportGuard::default();
    let _asset_guard = AssetConfigDropGuard::new();

    let warning_messages = prettier_build(cmd)?;

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

    println!("build desktop done");

    Ok(BuildResult {
        warnings: warning_messages.warnings,
        executable: Some(output_path),
        elapsed_time: t_start.elapsed(),
        assets,
    })
}

/// Build client (WASM).
/// Note: `rust_flags` argument is only used for the fullstack platform.
pub fn build_web(
    config: &CrateConfig,
    skip_assets: bool,
    rust_flags: Option<String>,
) -> Result<BuildResult> {
    // [1] Build the project with cargo, generating a wasm32-unknown-unknown target (is there a more specific, better target to leverage?)
    // [2] Generate the appropriate build folders
    // [3] Wasm-bindgen the .wasm file, and move it into the {builddir}/modules/xxxx/xxxx_bg.wasm
    // [4] Wasm-opt the .wasm file with whatever optimizations need to be done
    // [5][OPTIONAL] Builds the Tailwind CSS file using the Tailwind standalone binary
    // [6] Link up the html page to the wasm module

    let CrateConfig {
        crate_dir,
        target_dir,
        dioxus_config,
        ..
    } = config;
    let out_dir = self.config.out_dir();

    let _asset_guard = AssetConfigDropGuard::new();
    let _manganis_support = ManganisSupportGuard::default();

    let t_start = std::time::Instant::now();
    let _guard = dioxus_cli_config::__private::save_config(config);

    // [1] Build the .wasm module
    tracing::info!("🚅 Running build command...");

    check_wasm_target()?;

    let mut cargo_args = vec!["--target".to_string(), "wasm32-unknown-unknown".to_string()];

    let mut cmd = subprocess::Exec::cmd("cargo")
        .set_rust_flags(rust_flags)
        .env("CARGO_TARGET_DIR", target_dir)
        .cwd(crate_dir)
        .arg("build")
        .arg("--message-format=json-render-diagnostics");

    // TODO: make the initial variable mutable to simplify all the expressions
    // below. Look inside the `build_desktop()` as an example.
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
    let CargoBuildResult {
        warnings,
        output_location,
    } = prettier_build(cmd)?;

    // Start Manganis linker intercept.
    let linker_args = vec![format!("{}", self.config.out_dir().display())];

    manganis_cli_support::start_linker_intercept(
        &LinkCommand::command_name(),
        cargo_args,
        Some(linker_args),
    )
    .unwrap();

    // this code will copy all public file to the output dir
    copy_assets_dir(config, dioxus_cli_config::Platform::Web)?;

    let assets = if !skip_assets {
        tracing::info!("Processing assets");
        let assets = asset_manifest(config);
        process_assets(config, &assets)?;
        Some(assets)
    } else {
        None
    };

    Ok(BuildResult {
        warnings,
        executable: Some(output_location),
        elapsed_time: t_start.elapsed(),
        assets,
    })
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

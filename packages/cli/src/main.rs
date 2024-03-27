use dioxus_cli_config::DioxusConfig;
use std::{env, path::PathBuf};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use anyhow::Context;
use clap::Parser;
use dioxus_cli::*;

use Commands::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::ERROR.into())
        .from_env()
        .unwrap()
        .add_directive("dioxus_cli=warn".parse().unwrap())
        .add_directive("manganis-cli-support=warn".parse().unwrap());

    // If RUST_LOG is set, default to env, otherwise filter to cli and manganis
    if env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt().init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    match args.action {
        Translate(opts) => opts
            .translate()
            .context(error_wrapper("Translation of HTML into RSX failed")),

        New(opts) => opts
            .create()
            .context(error_wrapper("Creating new project failed")),

        Init(opts) => opts
            .init()
            .context(error_wrapper("Initialising a new project failed")),

        Config(opts) => opts
            .config()
            .context(error_wrapper("Configuring new project failed")),

        #[cfg(feature = "plugin")]
        Plugin(opts) => opts
            .plugin()
            .await
            .context(error_wrapper("Error with plugin")),

        Autoformat(opts) => opts
            .autoformat()
            .context(error_wrapper("Error autoformatting RSX")),

        Check(opts) => opts
            .check()
            .await
            .context(error_wrapper("Error checking RSX")),

        action => {
            let bin = get_bin(args.bin)?;
            let _dioxus_config = DioxusConfig::load(Some(bin.clone()))
                .context("Failed to load Dioxus config because")?
                .unwrap_or_else(|| {
                    tracing::info!("You appear to be creating a Dioxus project from scratch; we will use the default config");
                    DioxusConfig::default()
                });

            #[cfg(feature = "plugin")]
            use dioxus_cli::plugin::PluginManager;

            #[cfg(feature = "plugin")]
            PluginManager::init(_dioxus_config.plugin)
                .context(error_wrapper("Plugin system initialization failed"))?;

            match action {
                Build(opts) => opts
                    .build(Some(bin.clone()), None, None)
                    .context(error_wrapper("Building project failed")),

                Clean(opts) => opts
                    .clean(Some(bin.clone()))
                    .context(error_wrapper("Cleaning project failed")),

                Serve(opts) => opts
                    .serve(Some(bin.clone()))
                    .await
                    .context(error_wrapper("Serving project failed")),

                Bundle(opts) => opts
                    .bundle(Some(bin.clone()))
                    .context(error_wrapper("Bundling project failed")),

                _ => unreachable!(),
            }
        }
    }
}

fn get_bin(bin: Option<String>) -> Result<PathBuf> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .map_err(Error::CargoMetadata)?;
    let package = if let Some(bin) = bin {
        metadata
            .workspace_packages()
            .into_iter()
            .find(|p| p.name == bin)
            .ok_or(Error::CargoError(format!("no such package: {}", bin)))?
    } else {
        metadata
            .root_package()
            .ok_or(Error::CargoError("no root package?".to_string()))?
    };

    let crate_dir = package
        .manifest_path
        .parent()
        .ok_or(Error::CargoError("couldn't take parent dir".to_string()))?;

    Ok(crate_dir.into())
}

/// Simplifies error messages that use the same pattern.
fn error_wrapper(message: &str) -> String {
    format!("🚫 {message}:")
}

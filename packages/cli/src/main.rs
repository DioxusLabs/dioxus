use dioxus_cli_config::DioxusConfig;
use std::{env, path::PathBuf};
use tracing_subscriber::EnvFilter;

use anyhow::{anyhow, Context};
use clap::Parser;
use dioxus_cli::{
    plugin::{get_dependency_paths, init_plugins, save_plugin_config},
    *,
};
use Commands::*;

const LOG_ENV: &str = "DIOXUS_LOG";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // If {LOG_ENV} is set, default to env, otherwise filter to cli
    // and manganis warnings and errors from other crates
    let mut filter = EnvFilter::new("error,dx=info,dioxus-cli=info,manganis-cli-support=info");
    if env::var(LOG_ENV).is_ok() {
        filter = EnvFilter::from_env(LOG_ENV);
    }
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mut project_command = None;

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

            let dioxus_config = DioxusConfig::load(Some(bin.clone()))
          .map_err(|e| anyhow!("Failed to load Dioxus config because: {e}"))?
          .unwrap_or_else(|| {
            log::warn!("You appear to be creating a Dioxus project from scratch; we will use the default config");
            DioxusConfig::default()
          });

            let crate_dir = dioxus_cli_config::crate_root()?;
            let dependency_paths = get_dependency_paths(&crate_dir)?;
            init_plugins(&dioxus_config, &crate_dir, &dependency_paths).await?;

            let out = match other {
                Build(opts) => opts
                    .build(Some(bin.clone()), None, None)
                    .await
                    .map_err(|e| anyhow!("🚫 Building project failed: {}", e)),
                Clean(opts) => opts
                    .clean(Some(bin.clone()))
                    .map_err(|e| anyhow!("🚫 Cleaning project failed: {}", e)),
                Serve(opts) => opts
                    .serve(Some(bin.clone()))
                    .await
                    .map_err(|e| anyhow!("🚫 Serving project failed: {}", e)),
                Plugin(opts) => opts
                    .plugin(&dioxus_config, &crate_dir, &dependency_paths)
                    .await
                    .map_err(|e| anyhow!("🚫 Plugin manager failed: {}", e)),
                Bundle(opts) => opts
                    .bundle(Some(bin.clone()))
                    .await
                    .map_err(|e| anyhow!("🚫 Bundling project failed: {}", e)),
                _ => unreachable!("Caught by previous match"),
            };
            project_command = Some(bin);
            out
        }
    }?;

    if let Some(bin) = project_command {
        save_plugin_config(bin).await?;
    }

    Ok(())
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

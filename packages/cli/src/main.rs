use dioxus_cli_config::DioxusConfig;
use std::path::PathBuf;

use anyhow::{anyhow, Context};
use clap::Parser;
use dioxus_cli::{
    plugin::{init_plugins, save_plugin_config},
    *,
};
use Commands::*;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    set_up_logging();

    let bin = get_bin(args.bin)?;

    let dioxus_config = DioxusConfig::load(Some(bin.clone()))
        .map_err(|e| anyhow!("Failed to load Dioxus config because: {e}"))?
        .unwrap_or_else(|| {
            log::warn!("You appear to be creating a Dioxus project from scratch; we will use the default config");
            DioxusConfig::default()
        });

    let crate_dir = dioxus_cli_config::crate_root()?;
    init_plugins(&dioxus_config, &crate_dir).await?;

    match args.action {
        Translate(opts) => opts
            .translate()
            .await
            .map_err(|e| anyhow!("🚫 Translation of HTML into RSX failed: {}", e)),

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

        Create(opts) => opts
            .create()
            .context(error_wrapper("Creating new project failed")),

        Init(opts) => opts
            .init()
            .context(error_wrapper("Initialising a new project failed")),

        Config(opts) => opts
            .config()
            .map_err(|e| anyhow!("🚫 Configuring new project failed: {}", e)),

        Plugin(opts) => opts
            .plugin(&dioxus_config)
            .await
            .map_err(|e| anyhow!("🚫 Plugin manager failed: {}", e)),

        Bundle(opts) => opts
            .bundle(Some(bin.clone()))
            .await
            .map_err(|e| anyhow!("🚫 Bundling project failed: {}", e)),

        Autoformat(opts) => opts
            .autoformat()
            .await
            .context(error_wrapper("Error autoformatting RSX")),

        Check(opts) => opts
            .check()
            .await
            .context(error_wrapper("Error checking RSX")),

        Version(opt) => {
            let version = opt.version();
            println!("{}", version);

            Ok(())
        }
    }?;

    save_plugin_config(bin).await?;

    Ok(())
}

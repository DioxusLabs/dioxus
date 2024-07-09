use std::{env, path::PathBuf};
use tracing_subscriber::EnvFilter;

use anyhow::Context;
use clap::Parser;
use dioxus_cli::*;

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

    match args.action {
        Translate(opts) => opts
            .translate()
            .context(error_wrapper("Translation of HTML into RSX failed")),

        New(opts) => opts
            .create()
            .context(error_wrapper("Creating new project failed")),

        Init(opts) => opts
            .init()
            .context(error_wrapper("Initializing a new project failed")),

        Config(opts) => opts
            .config()
            .context(error_wrapper("Configuring new project failed")),

        Autoformat(opts) => opts
            .autoformat()
            .context(error_wrapper("Error autoformatting RSX")),

        Check(opts) => opts
            .check()
            .await
            .context(error_wrapper("Error checking RSX")),

        Link(opts) => opts
            .link()
            .context(error_wrapper("Error with linker passthrough")),

        action => {
            let bin = get_bin(args.bin)?;
            let dioxus_crate =
                DioxusCrate::new(Some(bin.clone())).context("Failed to load Dioxus workspace")?;

            match action {
                Build(mut opts) => opts
                    .build(dioxus_crate)
                    .await
                    .context(error_wrapper("Building project failed")),

                Clean(opts) => opts
                    .clean(Some(bin.clone()))
                    .context(error_wrapper("Cleaning project failed")),

                Serve(opts) => opts
                    .serve(dioxus_crate)
                    .await
                    .context(error_wrapper("Serving project failed")),

                Bundle(opts) => opts
                    .bundle(dioxus_crate)
                    .await
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

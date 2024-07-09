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
            let mut dioxus_crate = DioxusCrate::new(None, None, None, Vec::new())
                .context("Failed to load Dioxus workspace")?;

            match action {
                Build(mut opts) => opts
                    .build(&mut dioxus_crate)
                    .await
                    .context(error_wrapper("Building project failed")),

                Clean(opts) => opts
                    .clean(dioxus_crate)
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

/// Simplifies error messages that use the same pattern.
fn error_wrapper(message: &str) -> String {
    format!("🚫 {message}:")
}

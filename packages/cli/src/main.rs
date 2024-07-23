use std::env;
use tracing_subscriber::{prelude::*, EnvFilter, Layer};

use anyhow::Context;
use clap::Parser;
use dioxus_cli::*;

use Commands::*;

const LOG_ENV: &str = "DIOXUS_LOG";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    build_tracing();

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

        Build(mut opts) => opts
            .run()
            .await
            .context(error_wrapper("Building project failed")),

        Clean(opts) => opts
            .clean()
            .context(error_wrapper("Cleaning project failed")),

        Serve(opts) => opts
            .serve()
            .await
            .context(error_wrapper("Serving project failed")),

        Bundle(opts) => opts
            .bundle()
            .await
            .context(error_wrapper("Bundling project failed")),
    }
}

/// Simplifies error messages that use the same pattern.
fn error_wrapper(message: &str) -> String {
    format!("🚫 {message}:")
}

fn build_tracing() {
    // If {LOG_ENV} is set, default to env, otherwise filter to cli
    // and manganis warnings and errors from other crates
    let mut filter = EnvFilter::new("error,dx=info,dioxus-cli=info,manganis-cli-support=info");
    if env::var(LOG_ENV).is_ok() {
        filter = EnvFilter::from_env(LOG_ENV);
    }

    let sub =
        tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().with_filter(filter));

    #[cfg(feature = "tokio-console")]
    sub.with(console_subscriber::spawn()).init();

    #[cfg(not(feature = "tokio-console"))]
    sub.init();
}

#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub(crate) mod assets;
pub(crate) mod build_info;
pub(crate) mod builder;
pub(crate) mod bundle_utils;
pub(crate) mod bundler;
pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod dioxus_crate;
pub(crate) mod error;
pub(crate) mod fastfs;
pub(crate) mod metadata;
pub(crate) mod serve;
pub(crate) mod settings;
pub(crate) mod tooling;
pub(crate) mod tracer;

pub(crate) use builder::Platform;
pub(crate) use cli::*;
pub(crate) use dioxus_crate::*;
pub(crate) use error::*;
pub(crate) use settings::*;
pub(crate) use tracer::{TraceMsg, TraceSrc};

use anyhow::Context;
use clap::Parser;
use Commands::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // If we have a magic env var set, we want to operate as a linker instead.
    if link::should_link() {
        return link::dump_link_args();
    }

    // Start the tracer so it captures logs from the build engine before we start the builder
    crate::serve::TraceController::initialize();

    let args = Cli::parse();
    match args.action {
        Translate(opts) => opts
            .translate()
            .context("⛔️ Translation of HTML into RSX failed:"),

        New(opts) => opts.create().context("🚫 Creating new project failed:"),

        Init(opts) => opts.init().context("🚫 Initializing a new project failed:"),

        Config(opts) => opts.config().context("🚫 Configuring new project failed:"),

        Autoformat(opts) => opts.autoformat().context("🚫 Error autoformatting RSX:"),

        Check(opts) => opts.check().await.context("🚫 Error checking RSX:"),

        Clean(opts) => opts.clean().context("🚫 Cleaning project failed:"),

        Build(opts) => opts.run().await.context("🚫 Building project failed:"),

        Serve(opts) => opts.serve().await.context("🚫 Serving project failed:"),

        Bundle(opts) => opts.bundle().await.context("🚫 Bundling project failed:"),

        Run(opts) => opts.run().await.context("🚫 Running project failed:"),

        HttpServer(opts) => opts.serve().await.context("🚫 Serving project failed:"),

        Doctor(opts) => opts.run().await.context("🚫 Checking project failed:"),
    }
}

#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

pub mod assets;
pub mod builder;
pub mod bundle_utils;
pub mod bundler;
pub mod cli;
pub mod config;
pub mod dioxus_crate;
pub mod dx_build_info;
pub mod error;
pub mod fastfs;
pub mod metadata;
pub mod serve;
pub mod settings;
pub mod tools;
pub mod tracer;

pub use bundler::AppBundle;
pub use cli::*;
pub use dioxus_crate::*;
pub use error::*;
pub use settings::*;

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

        HttpServer(opts) => opts.serve().await.context("🚫 Serving project failed:"),
    }
}

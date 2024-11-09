#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod assets;
mod builder;
mod bundle_utils;
mod cli;
mod config;
mod dioxus_crate;
mod dx_build_info;
mod error;
mod fastfs;
mod filemap;
mod logging;
mod metadata;
mod platform;
mod profiles;
mod rustup;
mod serve;
mod settings;
mod slog;
mod tooling;

pub(crate) use builder::*;
pub(crate) use cli::*;
pub(crate) use config::*;
pub(crate) use dioxus_crate::*;
pub(crate) use dioxus_dx_wire_format::*;
pub(crate) use error::*;
pub(crate) use filemap::*;
pub(crate) use logging::*;
pub(crate) use platform::*;
pub(crate) use rustup::*;
pub(crate) use settings::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use anyhow::Context;
    use Commands::*;

    // If we're being ran as a linker (likely from ourselves), we want to act as a linker instead.
    if let Some(link_action) = link::LinkAction::from_env() {
        return link_action.run();
    }

    let args = TraceController::initialize();
    let result = match args.action {
        Translate(opts) => opts
            .translate()
            .context("⛔️ Translation of HTML into RSX failed:"),

        New(opts) => opts.create().context("🚫 Creating new project failed:"),

        Init(opts) => opts.init().context("🚫 Initializing a new project failed:"),

        Config(opts) => opts.config().context("🚫 Configuring new project failed:"),

        Autoformat(opts) => opts.autoformat().context("🚫 Error autoformatting RSX:"),

        Check(opts) => opts.check().await.context("🚫 Error checking RSX:"),

        Clean(opts) => opts.clean().context("🚫 Cleaning project failed:"),

        Build(opts) => opts.run_cmd().await.context("🚫 Building project failed:"),

        Serve(opts) => opts.serve().await.context("🚫 Serving project failed:"),

        Bundle(opts) => opts.bundle().await.context("🚫 Bundling project failed:"),

        Run(opts) => opts.run().await.context("🚫 Running project failed:"),

        Doctor(opts) => opts.run().await.context("🚫 Checking project failed:"),
    };

    // Provide a structured output for third party tools that can consume the output of the CLI
    match result {
        Ok(output) => {
            tracing::debug!(structured = ?output);
            Ok(())
        }
        Err(err) => {
            tracing::debug!(structured = ?err);
            Err(err)
        }
    }
}

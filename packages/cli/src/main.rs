#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::doc_overindented_list_items)]

mod build;
mod bundle_utils;
mod cargo_toml;
mod cli;
mod config;
mod devcfg;
mod dx_build_info;
mod error;
mod fastfs;
mod logging;
mod platform;
mod rustcwrapper;
mod serve;
mod settings;
mod tailwind;
mod test_harnesses;
mod wasm_bindgen;
mod wasm_opt;
mod workspace;

use std::process::ExitCode;

pub(crate) use build::*;
pub(crate) use cli::*;
pub(crate) use config::*;
pub(crate) use dioxus_dx_wire_format::*;
pub(crate) use error::*;
pub(crate) use link::*;
pub(crate) use logging::*;
pub(crate) use platform::*;
pub(crate) use rustcwrapper::*;
pub(crate) use settings::*;
pub(crate) use tailwind::*;
pub(crate) use wasm_bindgen::*;
pub(crate) use workspace::*;

#[tokio::main]
async fn main() -> ExitCode {
    // The CLI uses dx as a rustcwrapper in some instances (like binary patching)
    if rustcwrapper::is_wrapping_rustc() {
        return rustcwrapper::run_rustc();
    }

    // If we're being ran as a linker (likely from ourselves), we want to act as a linker instead.
    if let Some(link_args) = link::LinkAction::from_env() {
        return link_args.run_link();
    }

    // Run under the tracing collector so we can capture errors/panics.
    let result = TraceController::main(|args, tracer| async move {
        match args {
            Commands::Serve(opts) => opts.serve(&tracer).await,
            Commands::Translate(opts) => opts.translate(),
            Commands::New(opts) => opts.create().await,
            Commands::Init(opts) => opts.init().await,
            Commands::Config(opts) => opts.config().await,
            Commands::Autoformat(opts) => opts.autoformat().await,
            Commands::Check(opts) => opts.check().await,
            Commands::Build(opts) => opts.build().await,
            Commands::Bundle(opts) => opts.bundle().await,
            Commands::Run(opts) => opts.run().await,
            Commands::SelfUpdate(opts) => opts.self_update().await,
            Commands::Tools(BuildTools::BuildAssets(opts)) => opts.run().await,
            Commands::Tools(BuildTools::HotpatchTip(opts)) => opts.run().await,
            Commands::Doctor(opts) => opts.doctor().await,
            Commands::Print(opts) => opts.print().await,
            Commands::Components(opts) => opts.run().await,
            Commands::ShellCompletions(opts) => opts.generate_and_print(),
        }
    });

    // Print the structured output in JSON format for third-party tools to consume.
    // Make sure we do this as the last step so you can always `tail -1` it
    match result.await {
        StructuredOutput::Error { message } => {
            tracing::error!(json = %StructuredOutput::Error { message });
            std::process::exit(1);
        }

        output => tracing::info!(json = %output),
    }

    ExitCode::SUCCESS
}

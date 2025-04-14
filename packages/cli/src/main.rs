#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod build;
mod bundle_utils;
mod cli;
mod config;
mod dx_build_info;
mod error;
mod fastfs;
mod logging;
mod metadata;
mod platform;
mod rustcwrapper;
mod serve;
mod settings;
mod wasm_bindgen;
mod wasm_opt;
mod workspace;

pub(crate) use build::*;
pub(crate) use cli::*;
pub(crate) use config::*;
pub(crate) use dioxus_dx_wire_format::*;
pub(crate) use error::*;
pub(crate) use link::*;
pub(crate) use logging::*;
pub(crate) use platform::*;
pub(crate) use settings::*;
pub(crate) use workspace::*;

#[tokio::main]
async fn main() {
    // The CLI uses dx as a rustcwrapper in some instances (like binary patching)
    if rustcwrapper::is_rustc() {
        return rustcwrapper::run_rustc().await;
    }

    // If we're being ran as a linker (likely from ourselves), we want to act as a linker instead.
    if let Some(link_action) = link::LinkAction::from_env() {
        return link_action.run().await.unwrap();
    }

    let args = TraceController::initialize();

    #[cfg(debug_assertions)]
    tracing::warn!("CLI was built with debug profile. Commands will run slower.");

    let result = match args.action {
        Commands::Translate(opts) => opts.translate(),
        Commands::New(opts) => opts.create(),
        Commands::Init(opts) => opts.init(),
        Commands::Config(opts) => opts.config().await,
        Commands::Autoformat(opts) => opts.autoformat().await,
        Commands::Check(opts) => opts.check().await,
        Commands::Clean(opts) => opts.clean().await,
        Commands::Build(opts) => opts.build().await,
        Commands::Serve(opts) => opts.serve().await,
        Commands::Bundle(opts) => opts.bundle().await,
        Commands::Run(opts) => opts.run().await,
    };

    // Provide a structured output for third party tools that can consume the output of the CLI
    match result {
        Ok(output) => {
            tracing::debug!(json = ?output);
        }
        Err(err) => {
            tracing::error!(
                ?err,
                json = ?StructuredOutput::Error {
                    message: format!("{err:?}"),
                },
            );

            std::process::exit(1);
        }
    };
}

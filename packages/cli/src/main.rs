#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::doc_overindented_list_items)]

mod build;
mod bundle_utils;
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
mod wasm_bindgen;
mod wasm_opt;
mod workspace;

pub(crate) use build::{BuildArtifacts, BuildRequest, BuilderUpdate, ProgressRx, ProgressTx, BuildId, AndroidTools, BuildContext, AppBuilder, BuildMode, extract_assets_from_file, HotpatchModuleCache, pre_render_static_routes};
pub(crate) use cli::{link, styles, TargetArgs, BuildArgs, platform_override, Cli, Commands, Verbosity, print, BuildTools, ServeArgs, BuildTargets, VERSION, update};
pub(crate) use config::{DioxusConfig, WasmOptConfig, CustomSignCommandSettings, DebianSettings, MacOsSettings, NSISInstallerMode, NsisSettings, PackageType, WebviewInstallMode, WindowsSettings, WixSettings, AddressArguments, AndroidSettings};
pub(crate) use dioxus_dx_wire_format::{BuildStage, StructuredOutput};
pub(crate) use error::{Result, Error};
pub(crate) use link::{LinkAction, LinkerFlavor};
pub(crate) use logging::{TraceSrc, Anonymized, TraceController, TraceContent, TraceMsg, VERBOSITY};
pub(crate) use platform::{BundleFormat, Renderer, TargetAlias, Platform, RendererArg};
pub(crate) use rustcwrapper::{RustcArgs, DX_RUSTC_WRAPPER_ENV_VAR};
pub(crate) use settings::CliSettings;
pub(crate) use tailwind::TailwindCli;
pub(crate) use wasm_bindgen::WasmBindgen;
pub(crate) use workspace::Workspace;

#[tokio::main]
async fn main() {
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
}

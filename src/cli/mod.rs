use anyhow::{Context, Result};
use std::path::PathBuf;
use structopt::StructOpt;

pub mod build;
pub mod cfg;
pub mod clean;
pub mod config;
pub mod serve;
pub mod translate;
pub mod watch;

/// Build, bundle & ship your Rust WASM application to the web.
#[derive(StructOpt)]
#[structopt(name = "trunk")]
pub struct Trunk {
    #[structopt(subcommand)]
    pub action: TrunkSubcommands,

    /// Path to the Trunk config file [default: Trunk.toml]
    #[structopt(long, parse(from_os_str), env = "TRUNK_CONFIG")]
    pub config: Option<PathBuf>,

    /// Enable verbose logging.
    #[structopt(short)]
    pub v: bool,
}

#[derive(StructOpt)]
pub enum TrunkSubcommands {
    /// Build the Rust WASM app and all of its assets.
    Build(build::Build),

    /// Translate some source file into Dioxus code.
    Translate(translate::Translate),

    /// Build, watch & serve the Rust WASM app and all of its assets.
    ///
    ///
    Serve(serve::Serve),

    /// Clean output artifacts.
    Clean(clean::Clean),

    /// Trunk config controls.
    Config(config::Config),
}

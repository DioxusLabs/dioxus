pub mod autoformat;
pub mod build;
pub mod bundle;
pub mod cfg;
pub mod check;
pub mod clean;
pub mod config;
pub mod create;
pub mod init;
pub mod plugin;
pub mod serve;
pub mod translate;
pub mod version;

use crate::{
    cfg::{ConfigOptsBuild, ConfigOptsServe},
    custom_error,
    error::Result,
    gen_page, server, Error,
};
use clap::{Parser, Subcommand};
use dioxus_cli_config::CrateConfig;
use html_parser::Dom;
use serde::Deserialize;
use std::{
    fmt::Display,
    fs::{remove_dir_all, File},
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

/// Build, Bundle & Ship Dioxus Apps.
#[derive(Parser)]
#[clap(name = "dioxus", version)]
pub struct Cli {
    #[clap(subcommand)]
    pub action: Commands,

    /// Enable verbose logging.
    #[clap(short)]
    pub v: bool,

    /// Specify bin target
    #[clap(global = true, long)]
    pub bin: Option<String>,
}

#[derive(Parser)]
pub enum Commands {
    /// Build the Rust WASM app and all of its assets.
    Build(build::Build),

    /// Translate some source file into Dioxus code.
    Translate(translate::Translate),

    /// Build, watch & serve the Rust WASM app and all of its assets.
    Serve(serve::Serve),

    /// Create a new project for Dioxus.
    Create(create::Create),

    /// Init a new project for Dioxus
    Init(init::Init),

    /// Clean output artifacts.
    Clean(clean::Clean),

    /// Bundle the Rust desktop app and all of its assets.
    Bundle(bundle::Bundle),

    /// Print the version of this extension
    #[clap(name = "version")]
    Version(version::Version),

    /// Format some rsx
    #[clap(name = "fmt")]
    Autoformat(autoformat::Autoformat),

    /// Check the Rust files in the project for issues.
    #[clap(name = "check")]
    Check(check::Check),

    /// Dioxus config file controls.
    #[clap(subcommand)]
    Config(config::Config),

    /// Manage plugins for dioxus cli
    #[cfg(feature = "plugin")]
    #[clap(subcommand)]
    Plugin(plugin::Plugin),
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::Build(_) => write!(f, "build"),
            Commands::Translate(_) => write!(f, "translate"),
            Commands::Serve(_) => write!(f, "serve"),
            Commands::Create(_) => write!(f, "create"),
            Commands::Init(_) => write!(f, "init"),
            Commands::Clean(_) => write!(f, "clean"),
            Commands::Config(_) => write!(f, "config"),
            Commands::Version(_) => write!(f, "version"),
            Commands::Autoformat(_) => write!(f, "fmt"),
            Commands::Check(_) => write!(f, "check"),
            Commands::Bundle(_) => write!(f, "bundle"),

            #[cfg(feature = "plugin")]
            Commands::Plugin(_) => write!(f, "plugin"),
        }
    }
}

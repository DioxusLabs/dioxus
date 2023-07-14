pub mod autoformat;
pub mod build;
pub mod cfg;
pub mod clean;
pub mod config;
pub mod create;
pub mod plugin;
pub mod serve;
pub mod translate;
pub mod version;

use crate::{
    cfg::{ConfigOptsBuild, ConfigOptsServe},
    custom_error,
    error::Result,
    gen_page, server, CrateConfig, Error,
};
use clap::{Parser, Subcommand};
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
}

#[derive(Parser)]
pub enum Commands {
    /// Build the Rust WASM app and all of its assets.
    Build(build::Build),

    /// Translate some source file into Dioxus code.
    Translate(translate::Translate),

    /// Build, watch & serve the Rust WASM app and all of its assets.
    Serve(serve::Serve),

    /// Init a new project for Dioxus.
    Create(create::Create),

    /// Clean output artifacts.
    Clean(clean::Clean),

    /// Print the version of this extension
    #[clap(name = "version")]
    Version(version::Version),

    /// Format some rsx
    #[clap(name = "fmt")]
    Autoformat(autoformat::Autoformat),

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
            Commands::Clean(_) => write!(f, "clean"),
            Commands::Config(_) => write!(f, "config"),
            Commands::Version(_) => write!(f, "version"),
            Commands::Autoformat(_) => write!(f, "fmt"),

            #[cfg(feature = "plugin")]
            Commands::Plugin(_) => write!(f, "plugin"),
        }
    }
}

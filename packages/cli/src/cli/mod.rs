pub mod autoformat;
pub mod build;
pub mod bundle;
pub mod check;
pub mod clean;
pub mod config;
pub mod create;
pub mod init;
pub mod link;
pub mod serve;
pub mod translate;

use crate::{custom_error, error::Result, Error};
use clap::{Parser, Subcommand};
use html_parser::Dom;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    fmt::Display,
    fs::{remove_dir_all, File},
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

pub static VERSION: Lazy<String> = Lazy::new(|| {
    format!(
        "{} ({})",
        crate::dx_build_info::PKG_VERSION,
        crate::dx_build_info::GIT_COMMIT_HASH_SHORT.unwrap_or("was built without git repository")
    )
});

/// Build, Bundle & Ship Dioxus Apps.
#[derive(Parser)]
#[clap(name = "dioxus", version = VERSION.as_str())]
pub struct Cli {
    #[clap(subcommand)]
    pub action: Commands,

    /// Enable verbose logging.
    #[clap(short)]
    pub v: bool,

    /// Specify a binary target.
    #[clap(global = true, long)]
    pub bin: Option<String>,
}

#[derive(Parser)]
pub enum Commands {
    /// Build the Dioxus project and all of its assets.
    Build(build::Build),

    /// Translate a source file into Dioxus code.
    Translate(translate::Translate),

    /// Build, watch & serve the Dioxus project and all of its assets.
    Serve(serve::Serve),

    /// Create a new project for Dioxus.
    New(create::Create),

    /// Init a new project for Dioxus in an existing directory.
    /// Will attempt to keep your project in a good state.
    Init(init::Init),

    /// Clean output artifacts.
    Clean(clean::Clean),

    /// Bundle the Dioxus app into a shippable object.
    Bundle(bundle::Bundle),

    /// Automatically format RSX.
    #[clap(name = "fmt")]
    Autoformat(autoformat::Autoformat),

    /// Check the project for any issues.
    #[clap(name = "check")]
    Check(check::Check),

    /// Dioxus config file controls.
    #[clap(subcommand)]
    Config(config::Config),

    /// Handles parsing of linker arguments for linker-based systems
    /// such as Manganis and binary patching.
    Link(link::LinkCommand),
}

impl Display for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::Build(_) => write!(f, "build"),
            Commands::Translate(_) => write!(f, "translate"),
            Commands::Serve(_) => write!(f, "serve"),
            Commands::New(_) => write!(f, "create"),
            Commands::Init(_) => write!(f, "init"),
            Commands::Clean(_) => write!(f, "clean"),
            Commands::Config(_) => write!(f, "config"),
            Commands::Autoformat(_) => write!(f, "fmt"),
            Commands::Check(_) => write!(f, "check"),
            Commands::Bundle(_) => write!(f, "bundle"),
            Commands::Link(_) => write!(f, "link"),
        }
    }
}

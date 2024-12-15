pub mod autoformat;
pub mod build;
pub mod bundle;
pub mod check;
pub mod clean;
pub mod config;
pub mod create;
pub mod doctor;
pub mod init;
pub mod link;
pub mod run;
pub mod serve;
pub mod target;
pub mod translate;
pub mod verbosity;

pub use build::*;
pub use serve::*;
pub use target::*;
pub use verbosity::*;

use crate::{error::Result, Error, StructuredOutput};
use anyhow::Context;
use clap::{Parser, Subcommand};
use html_parser::Dom;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

/// Build, Bundle & Ship Dioxus Apps.
#[derive(Parser)]
#[clap(name = "dioxus", version = VERSION.as_str())]
pub struct Cli {
    #[command(subcommand)]
    pub action: Commands,

    #[command(flatten)]
    pub verbosity: Verbosity,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build the Dioxus project and all of its assets.
    #[clap(name = "build")]
    Build(build::BuildArgs),

    /// Translate a source file into Dioxus code.
    #[clap(name = "translate")]
    Translate(translate::Translate),

    /// Build, watch & serve the Dioxus project and all of its assets.
    #[clap(name = "serve")]
    Serve(serve::ServeArgs),

    /// Create a new project for Dioxus.
    #[clap(name = "new")]
    New(create::Create),

    /// Init a new project for Dioxus in the current directory (by default).
    /// Will attempt to keep your project in a good state.
    #[clap(name = "init")]
    Init(init::Init),

    /// Clean output artifacts.
    #[clap(name = "clean")]
    Clean(clean::Clean),

    /// Bundle the Dioxus app into a shippable object.
    #[clap(name = "bundle")]
    Bundle(bundle::Bundle),

    /// Automatically format RSX.
    #[clap(name = "fmt")]
    Autoformat(autoformat::Autoformat),

    /// Check the project for any issues.
    #[clap(name = "check")]
    Check(check::Check),

    /// Run the project without any hotreloading
    #[clap(name = "run")]
    Run(run::RunArgs),

    /// Ensure all the tooling is installed and configured correctly
    #[clap(name = "doctor")]
    Doctor(doctor::Doctor),

    /// Dioxus config file controls.
    #[clap(subcommand)]
    #[clap(name = "config")]
    Config(config::Config),
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
            Commands::Run(_) => write!(f, "run"),
            Commands::Doctor(_) => write!(f, "doctor"),
        }
    }
}

static VERSION: Lazy<String> = Lazy::new(|| {
    format!(
        "{} ({})",
        crate::dx_build_info::PKG_VERSION,
        crate::dx_build_info::GIT_COMMIT_HASH_SHORT.unwrap_or("was built without git repository")
    )
});

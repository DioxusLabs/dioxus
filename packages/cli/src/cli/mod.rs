pub(crate) mod autoformat;
pub(crate) mod build;
pub(crate) mod bundle;
pub(crate) mod check;
pub(crate) mod clean;
pub(crate) mod config;
pub(crate) mod create;
pub(crate) mod init;
pub(crate) mod link;
pub(crate) mod run;
pub(crate) mod serve;
pub(crate) mod target;
pub(crate) mod translate;
pub(crate) mod verbosity;

pub(crate) use build::*;
pub(crate) use serve::*;
pub(crate) use target::*;
pub(crate) use verbosity::*;

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
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) action: Commands,

    #[command(flatten)]
    pub(crate) verbosity: Verbosity,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
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
        }
    }
}

pub(crate) static VERSION: Lazy<String> = Lazy::new(|| {
    format!(
        "{} ({})",
        crate::dx_build_info::PKG_VERSION,
        crate::dx_build_info::GIT_COMMIT_HASH_SHORT.unwrap_or("was built without git repository")
    )
});

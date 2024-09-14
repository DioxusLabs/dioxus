pub(crate) mod autoformat;
pub(crate) mod build;
pub(crate) mod bundle;
pub(crate) mod check;
pub(crate) mod clean;
pub(crate) mod config;
pub(crate) mod create;
pub(crate) mod doctor;
pub(crate) mod httpserver;
pub(crate) mod init;
pub(crate) mod link;
pub(crate) mod run;
pub(crate) mod serve;
pub(crate) mod translate;

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

/// Build, Bundle & Ship Dioxus Apps.
#[derive(Parser)]
#[clap(name = "dioxus", version = VERSION.as_str())]
pub(crate) struct Cli {
    #[clap(subcommand)]
    pub(crate) action: Commands,

    /// Enable verbose logging.
    #[clap(short)]
    pub(crate) v: bool,

    /// Specify a binary target.
    #[clap(global = true, long)]
    pub(crate) bin: Option<String>,
}

#[derive(Parser)]
pub(crate) enum Commands {
    /// Build the Dioxus project and all of its assets.
    Build(build::BuildArgs),

    /// Translate a source file into Dioxus code.
    Translate(translate::Translate),

    /// Build, watch & serve the Dioxus project and all of its assets.
    Serve(serve::ServeArgs),

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

    /// Start a local http server, akin to a default fullstack app
    #[clap(name = "http-server")]
    HttpServer(httpserver::Httpserver),

    /// Run the project without any hotreloading
    #[clap(name = "run")]
    Run(run::RunArgs),

    /// Ensure all the tooling is installed and configured correctly
    #[clap(name = "doctor")]
    Doctor(doctor::Doctor),

    /// Dioxus config file controls.
    #[clap(subcommand)]
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
            Commands::HttpServer(_) => write!(f, "http-server"),
            Commands::Run(_) => write!(f, "run"),
            Commands::Doctor(_) => write!(f, "doctor"),
        }
    }
}

pub(crate) static VERSION: Lazy<String> = Lazy::new(|| {
    format!(
        "{} ({})",
        crate::build_info::PKG_VERSION,
        crate::build_info::GIT_COMMIT_HASH_SHORT.unwrap_or("was built without git repository")
    )
});

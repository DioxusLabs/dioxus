pub mod build;
pub mod cfg;
pub mod clean;
pub mod config;
pub mod create;
pub mod serve;
pub mod translate;

use std::{io::Write, path::PathBuf};
use crate::{cfg::ConfigOptsBuild, gen_page};
use clap::{Parser};
use serde::Deserialize;
use std::{
    fs::remove_dir_all,
    process::{Command, Stdio},
};
use std::{fs::File};
use std::{
    io::{Read},
};
use regex::Regex;
use crate::{error::Result, Error};
use crate::{cfg::ConfigOptsServe, server, CrateConfig};
use html_parser::Dom;
use html_parser::Element;
use html_parser::Node;
use std::fmt::{Display, Formatter};
use std::process::exit;

/// Build, bundle, & ship your Dioxus app.
///
///
#[derive(Parser, Debug)]
#[clap(name = "dioxus")]
pub struct Cli {
    #[clap(subcommand)]
    pub action: Commands,

    /// Enable verbose logging.
    #[clap(short)]
    pub v: bool,
    //
    // // note: dioxus is still roughly compatible with trunk
    // /// Path to the Trunk config file [default: Trunk.toml]
    // #[structopt(long, parse(from_os_str), env = "TRUNK_CONFIG")]
    // pub config: Option<PathBuf>,
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
    /// Dioxus config file controls.
    Config(config::Config),
}

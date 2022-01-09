use crate::cfg::ConfigOptsBuild;
use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;

pub mod extract_svgs;
pub mod to_component;
pub mod translate;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "translate")]
pub struct Translate {}

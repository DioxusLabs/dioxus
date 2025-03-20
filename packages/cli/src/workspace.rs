use crate::CliSettings;
use crate::{config::DioxusConfig, TargetArgs};
use crate::{Platform, Result};
use anyhow::Context;
use itertools::Itertools;
use krates::{cm::Target, KrateDetails};
use krates::{cm::TargetKind, Cmd, Krates, NodeId};
use once_cell::sync::OnceCell;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use target_lexicon::Triple;
use tokio::process::Command;
use toml_edit::Item;

// pub struct Workspace {
//     pub(crate) krates: Krates,
//     pub(crate) settings: CliSettings,
// }

// impl Workspace {
//     fn new() -> Result<Self> {}

//     fn get_target() {}
// }

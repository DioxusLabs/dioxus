use crate::config::DioxusConfig;
use crate::{CliSettings, RustcDetails};
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

pub struct Workspace {
    pub(crate) krates: Krates,
    pub(crate) settings: CliSettings,
    pub(crate) rustc: RustcDetails,
}

static WS: OnceCell<Arc<Workspace>> = OnceCell::new();

impl Workspace {
    pub async fn current() -> Result<Arc<Workspace>> {
        tracing::debug!("Loading workspace");
        let cmd = Cmd::new();
        let builder = krates::Builder::new();
        let krates = builder
            .build(cmd, |_| {})
            .context("Failed to run cargo metadata")?;

        let settings = CliSettings::global_or_default();
        let rustc = RustcDetails::from_cli().await?;

        Ok(Arc::new(Self {
            krates,
            settings,
            rustc,
        }))
    }

    pub fn wasm_ld(&self) -> PathBuf {
        self.rustc
            .sysroot
            .join("lib")
            .join("rustlib")
            .join(Triple::host().to_string())
            .join("bin")
            .join("gcc-ld")
            .join("wasm-ld")
    }
}

// pub struct Workspace {
//     pub(crate) krates: Krates,
//     pub(crate) settings: CliSettings,
//     pub(crate) rustc: RustcDetails,
// }

// impl Workspace {
//     pub async fn load() -> Result<Arc<Workspace>> {
//         tracing::debug!("Loading workspace");
//         let cmd = Cmd::new();
//         let builder = krates::Builder::new();
//         let krates = builder
//             .build(cmd, |_| {})
//             .context("Failed to run cargo metadata")?;

//         let settings = CliSettings::global_or_default();
//         let rustc = RustcDetails::from_cli().await?;

//         Ok(Arc::new(Self {
//             krates,
//             settings,
//             rustc,
//         }))
//     }

// }

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace")
            .field("krates", &"..")
            .field("settings", &self.settings)
            .field("rustc", &self.rustc)
            .finish()
    }
}

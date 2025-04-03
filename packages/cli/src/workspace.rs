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

pub struct Workspace {
    pub(crate) krates: Krates,
    pub(crate) settings: CliSettings,
    pub(crate) rustc: RustcDetails,
    pub(crate) wasm_opt: Option<PathBuf>,
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

        let wasm_opt = which::which("wasm-opt").ok();

        Ok(Arc::new(Self {
            krates,
            settings,
            rustc,
            wasm_opt,
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

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace")
            .field("krates", &"..")
            .field("settings", &self.settings)
            .field("rustc", &self.rustc)
            .finish()
    }
}

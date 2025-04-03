use crate::config::DioxusConfig;
use crate::CliSettings;
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
    pub(crate) wasm_opt: Option<PathBuf>,
    pub(crate) sysroot: PathBuf,
    pub(crate) rustc_version: String,
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
        let sysroot = Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .await
            .map(|out| String::from_utf8(out.stdout))?
            .context("Failed to extract rustc sysroot output")?;

        let rustc_version = Command::new("rustc")
            .args(["--version"])
            .output()
            .await
            .map(|out| String::from_utf8(out.stdout))?
            .context("Failed to extract rustc version output")?;

        let wasm_opt = which::which("wasm-opt").ok();

        Ok(Arc::new(Self {
            krates,
            settings,
            wasm_opt,
            sysroot: sysroot.trim().into(),
            rustc_version: rustc_version.trim().into(),
        }))
    }

    pub fn wasm_ld(&self) -> PathBuf {
        self.sysroot
            .join("lib")
            .join("rustlib")
            .join(Triple::host().to_string())
            .join("bin")
            .join("gcc-ld")
            .join("wasm-ld")
    }

    pub fn has_wasm32_unknown_unknown(&self) -> bool {
        self.sysroot
            .join("lib/rustlib/wasm32-unknown-unknown")
            .exists()
    }
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace")
            .field("krates", &"..")
            .field("settings", &self.settings)
            .field("rustc_version", &self.rustc_version)
            .field("sysroot", &self.sysroot)
            .field("wasm_opt", &self.wasm_opt)
            .finish()
    }
}

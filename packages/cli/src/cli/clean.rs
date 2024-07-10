use crate::DioxusCrate;
use anyhow::Context;
use build::TargetArgs;

use super::*;

/// Clean build artifacts.
#[derive(Clone, Debug, Parser)]
#[clap(name = "clean")]
pub struct Clean {}

impl Clean {
    pub fn clean(self) -> anyhow::Result<()> {
        let dioxus_crate =
            DioxusCrate::new(&TargetArgs::default()).context("Failed to load Dioxus workspace")?;

        let output = Command::new("cargo")
            .arg("clean")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Cargo clean failed."));
        }

        let out_dir = &dioxus_crate.out_dir();
        if out_dir.is_dir() {
            remove_dir_all(out_dir)?;
        }

        let fullstack_out_dir = dioxus_crate.fullstack_out_dir();

        if fullstack_out_dir.is_dir() {
            remove_dir_all(fullstack_out_dir)?;
        }

        Ok(())
    }
}

use super::*;
use crate::DioxusCrate;
use anyhow::Context;

/// Clean build artifacts.
#[derive(Clone, Debug, Parser)]
#[clap(name = "clean")]
pub(crate) struct Clean {}

impl Clean {
    pub(crate) fn clean(self) -> anyhow::Result<()> {
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

        todo!();
        // let out_dir = &dioxus_crate.out_dir();
        // if out_dir.is_dir() {
        //     remove_dir_all(out_dir)?;
        // }

        Ok(())
    }
}

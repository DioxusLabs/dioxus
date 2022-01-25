use std::{
    fs::remove_dir_all,
    path::PathBuf,
    process::{Command, Stdio},
};

use structopt::StructOpt;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "clean")]
pub struct Clean {}

impl Clean {
    pub fn clean(self) -> anyhow::Result<()> {
        let crate_config = crate::CrateConfig::new()?;

        let output = Command::new("cargo")
            .arg("clean")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            log::error!("Cargo clean failed.");
            return Ok(());
        }

        let out_dir = crate_config
            .dioxus_config
            .application
            .out_dir
            .unwrap_or_else(|| PathBuf::from("dist"));
        if crate_config.crate_dir.join(&out_dir).is_dir() {
            remove_dir_all(crate_config.crate_dir.join(&out_dir))?;
        }

        Ok(())
    }
}

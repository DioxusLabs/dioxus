use crate::Result;
use anyhow::Context;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Default)]
pub struct RustcDetails {
    pub sysroot: PathBuf,
    pub version: String,
}

impl RustcDetails {
    /// Find the current sysroot location using the CLI
    pub async fn from_cli() -> Result<RustcDetails> {
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

        Ok(Self {
            sysroot: sysroot.trim().into(),
            version: rustc_version.trim().into(),
        })
    }

    pub fn has_wasm32_unknown_unknown(&self) -> bool {
        self.sysroot
            .join("lib/rustlib/wasm32-unknown-unknown")
            .exists()
    }
}

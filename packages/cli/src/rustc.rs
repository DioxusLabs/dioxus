use crate::Result;
use anyhow::Context;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Default)]
pub struct RustcDetails {
    pub sysroot: PathBuf,
}

impl RustcDetails {
    /// Find the current sysroot location using the CLI
    pub async fn from_cli() -> Result<RustcDetails> {
        let output = Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .await?;

        let stdout =
            String::from_utf8(output.stdout).context("Failed to extract rustc sysroot output")?;

        let sysroot = PathBuf::from(stdout.trim());
        Ok(Self { sysroot })
    }

    pub fn has_wasm32_unknown_unknown(&self) -> bool {
        self.sysroot
            .join("lib/rustlib/wasm32-unknown-unknown")
            .exists()
    }
}

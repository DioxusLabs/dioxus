use super::WasmBindgenBinary;
use anyhow::{anyhow, Context};
use std::path::PathBuf;
use tokio::process::Command;

pub(super) struct PathBinary {
    version: String,
}

impl WasmBindgenBinary for PathBinary {
    fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
        }
    }

    async fn verify_install(&self) -> anyhow::Result<()> {
        tracing::info!(
            "Verifying wasm-bindgen-cli@{} is installed in the path",
            self.version
        );

        let binary = self.get_binary_path().await?;
        let output = Command::new(binary)
            .args(["--version"])
            .output()
            .await
            .context("Failed to check wasm-bindgen-cli version")?;

        let stdout = String::from_utf8(output.stdout)
            .context("Failed to extract wasm-bindgen-cli output")?;

        let installed_version = stdout.trim_start_matches("wasm-bindgen").trim();
        if installed_version != self.version {
            return Err(anyhow!(
                "Incorrect wasm-bindgen-cli version: project requires version {} but version {} is installed",
                self.version,
                installed_version,
            ));
        }

        Ok(())
    }

    async fn get_binary_path(&self) -> anyhow::Result<PathBuf> {
        which::which("wasm-bindgen")
            .map_err(|_| anyhow!("Missing wasm-bindgen-cli@{}", self.version))
    }
}

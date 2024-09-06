use crate::{Error, Result};
use tokio::process::Command;

pub struct ToolingProvider {}

impl ToolingProvider {
    /// Check if the wasm32-unknown-unknown target is installed and try to install it if not
    pub(crate) async fn install_web_build_tooling(&self) -> Result<()> {
        // If the user has rustup, we can check if the wasm32-unknown-unknown target is installed
        // Otherwise we can just assume it is installed - which is not great...
        // Eventually we can poke at the errors and let the user know they need to install the target
        if let Ok(wasm_check_command) = Command::new("rustup").args(["show"]).output().await {
            let wasm_check_output = String::from_utf8(wasm_check_command.stdout).unwrap();
            if !wasm_check_output.contains("wasm32-unknown-unknown") {
                // _ = self.progress.unbounded_send(BuildUpdateProgress {
                //     stage: Stage::InstallingWasmTooling,
                //     update: UpdateStage::Start,
                //     platform: self.platform(),
                // });
                tracing::info!("wasm32-unknown-unknown target not detected, installing..");
                let _ = Command::new("rustup")
                    .args(["target", "add", "wasm32-unknown-unknown"])
                    .output()
                    .await?;
            }
        }

        Ok(())
    }

    // Attempt to automatically recover from a bindgen failure by updating the wasm-bindgen version
    pub(crate) async fn update_wasm_bindgen_version() -> Result<()> {
        let cli_bindgen_version = wasm_bindgen_shared::version();
        tracing::info!("Attempting to recover from bindgen failure by setting the wasm-bindgen version to {cli_bindgen_version}...");

        let output = Command::new("cargo")
            .args([
                "update",
                "-p",
                "wasm-bindgen",
                "--precise",
                &cli_bindgen_version,
            ])
            .output()
            .await;

        let mut error_message = None;
        if let Ok(output) = output {
            if output.status.success() {
                tracing::info!("Successfully updated wasm-bindgen to {cli_bindgen_version}");
                return Ok(());
            } else {
                error_message = Some(output);
            }
        }

        if let Some(output) = error_message {
            tracing::error!("Failed to update wasm-bindgen: {:#?}", output);
        }

        Err(Error::BuildFailed(format!("WASM bindgen build failed!\nThis is probably due to the Bindgen version, dioxus-cli is using `{cli_bindgen_version}` which is not compatible with your crate.\nPlease reinstall the dioxus cli to fix this issue.\nYou can reinstall the dioxus cli by running `cargo install dioxus-cli --force` and then rebuild your project")))
    }
}

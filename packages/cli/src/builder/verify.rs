use std::process::Stdio;

use crate::{BuildRequest, Platform, Result, RustupShow};
use anyhow::Context;
use tokio::process::Command;

impl BuildRequest {
    /// Install any tooling that might be required for this build.
    ///
    /// This should generally be only called on the first build since it takes time to verify the tooling
    /// is in place, and we don't want to slow down subsequent builds.
    pub(crate) async fn verify_tooling(&self) -> Result<()> {
        tracing::debug!("Verifying tooling...");
        self.status_installing_tooling();

        self.krate
            .initialize_profiles()
            .context("Failed to initialize profiles - dioxus can't build without them. You might need to initialize them yourself.")?;

        let rustup = match RustupShow::from_cli().await {
            Ok(out) => out,
            Err(err) => {
                tracing::error!("Failed to verify tooling: {err}\ndx will proceed, but you might run into errors later.");
                return Ok(());
            }
        };

        match self.build.platform() {
            Platform::Web => self.verify_web_tooling(rustup).await?,
            Platform::Ios => self.verify_ios_tooling(rustup).await?,
            Platform::Android => self.verify_android_tooling(rustup).await?,
            Platform::Linux => self.verify_linux_tooling(rustup).await?,
            Platform::MacOS => {}
            Platform::Windows => {}
            Platform::Server => {}
            Platform::Liveview => {}
        }

        Ok(())
    }

    pub(crate) async fn verify_web_tooling(&self, rustup: RustupShow) -> Result<()> {
        if !rustup.has_wasm32_unknown_unknown() {
            tracing::info!(
                "Web platform requires wasm32-unknown-unknown to be installed. Installing..."
            );
            let _ = Command::new("rustup")
                .args(["target", "add", "wasm32-unknown-unknown"])
                .output()
                .await?;
        }

        match self.krate.wasm_bindgen_version() {
            Some(version) if version == wasm_bindgen_shared::SCHEMA_VERSION  => {
                tracing::debug!("wasm-bindgen version {version} is compatible with dioxus-cli ✅");
            },
            Some(version) => {
                tracing::warn!(
                    "wasm-bindgen version {version} is not compatible with the cli crate. Attempting to upgrade the target wasm-bindgen crate manually..."
                );

                let output = Command::new("cargo")
                    .args([
                        "update",
                        "-p",
                        "wasm-bindgen",
                        "--precise",
                        &wasm_bindgen_shared::version(),
                    ])
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .output()
                    .await;

                match output {
                    Ok(output) if output.status.success() => tracing::info!("✅ wasm-bindgen updated successfully"),
                    Ok(output) => tracing::error!("Failed to update wasm-bindgen: {:?}", output),
                    Err(err) => tracing::error!("Failed to update wasm-bindgen: {err}"),
                }

            }
            None => tracing::debug!("User is attempting a web build without wasm-bindgen detected. This is probably a bug in the dioxus-cli."),
        }

        Ok(())
    }

    /// Currently does nothing, but eventually we need to check that the mobile tooling is installed.
    ///
    /// For ios, this would be just aarch64-apple-ios + aarch64-apple-ios-sim, as well as xcrun and xcode-select
    ///
    /// We don't auto-install these yet since we're not doing an architecture check. We assume most users
    /// are running on an Apple Silicon Mac, but it would be confusing if we installed these when we actually
    /// should be installing the x86 versions.
    pub(crate) async fn verify_ios_tooling(&self, _rustup: RustupShow) -> Result<()> {
        // open the simulator
        _ = tokio::process::Command::new("open")
            .arg("/Applications/Xcode.app/Contents/Developer/Applications/Simulator.app")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .status()
            .await;

        // Now xcrun to open the device
        // todo: we should try and query the device list and/or parse it rather than hardcode this simulator
        _ = tokio::process::Command::new("xcrun")
            .args(["simctl", "boot", "83AE3067-987F-4F85-AE3D-7079EF48C967"])
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .status()
            .await;

        // if !rustup
        //     .installed_toolchains
        //     .contains(&"aarch64-apple-ios".to_string())
        // {
        //     tracing::error!("You need to install aarch64-apple-ios to build for ios. Run `rustup target add aarch64-apple-ios` to install it.");
        // }

        // if !rustup
        //     .installed_toolchains
        //     .contains(&"aarch64-apple-ios-sim".to_string())
        // {
        //     tracing::error!("You need to install aarch64-apple-ios to build for ios. Run `rustup target add aarch64-apple-ios` to install it.");
        // }

        Ok(())
    }

    /// Check if the android tooling is installed
    ///
    /// looks for the android sdk + ndk
    ///
    /// will do its best to fill in the missing bits by exploring the sdk structure
    /// IE will attempt to use the Java installed from android studio if possible.
    pub(crate) async fn verify_android_tooling(&self, _rustup: RustupShow) -> Result<()> {
        Ok(())
    }

    /// Ensure the right dependencies are installed for linux apps.
    /// This varies by distro, so we just do nothing for now.
    ///
    /// Eventually, we want to check for the prereqs for wry/tao as outlined by tauri:
    ///     https://tauri.app/start/prerequisites/
    pub(crate) async fn verify_linux_tooling(&self, _rustup: crate::RustupShow) -> Result<()> {
        Ok(())
    }
}

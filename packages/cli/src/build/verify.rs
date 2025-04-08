use crate::{
    wasm_bindgen::WasmBindgen, BuildRequest, DioxusCrate, Error, Platform, Result, RustcDetails,
};
use anyhow::{anyhow, Context};

impl BuildRequest {
    /// Check for tooling that might be required for this build.
    ///
    /// This should generally be only called on the first build since it takes time to verify the tooling
    /// is in place, and we don't want to slow down subsequent builds.
    pub(crate) async fn verify_tooling(&self) -> Result<()> {
        tracing::debug!("Verifying tooling...");
        self.status_installing_tooling();

        self.krate
            .initialize_profiles()
            .context("Failed to initialize profiles - dioxus can't build without them. You might need to initialize them yourself.")?;

        let rustc = match RustcDetails::from_cli().await {
            Ok(out) => out,
            Err(err) => {
                tracing::error!("Failed to verify tooling: {err}\ndx will proceed, but you might run into errors later.");
                return Ok(());
            }
        };

        match self.build.platform() {
            Platform::Web => self.verify_web_tooling(rustc).await?,
            Platform::Ios => self.verify_ios_tooling(rustc).await?,
            Platform::Android => self.verify_android_tooling(rustc).await?,
            Platform::Linux => self.verify_linux_tooling(rustc).await?,
            Platform::MacOS => {}
            Platform::Windows => {}
            Platform::Server => {}
            Platform::Liveview => {}
        }

        Ok(())
    }

    pub(crate) async fn verify_web_tooling(&self, rustc: RustcDetails) -> Result<()> {
        // Install target using rustup.
        #[cfg(not(feature = "no-downloads"))]
        if !rustc.has_wasm32_unknown_unknown() {
            tracing::info!(
                "Web platform requires wasm32-unknown-unknown to be installed. Installing..."
            );

            let _ = tokio::process::Command::new("rustup")
                .args(["target", "add", "wasm32-unknown-unknown"])
                .output()
                .await?;
        }

        // Ensure target is installed.
        if !rustc.has_wasm32_unknown_unknown() {
            return Err(Error::Other(anyhow!(
                "Missing target wasm32-unknown-unknown."
            )));
        }

        // Wasm bindgen
        let krate_bindgen_version = self.krate.wasm_bindgen_version().ok_or(anyhow!(
            "failed to detect wasm-bindgen version, unable to proceed"
        ))?;

        WasmBindgen::verify_install(&krate_bindgen_version).await?;

        Ok(())
    }

    /// Verify that the required iOS tooling is installed.
    ///
    /// This checks for the appropriate iOS target based on the host architecture:
    /// - For device builds: aarch64-apple-ios
    /// - For simulator builds on Intel Macs: x86_64-apple-ios
    /// - For simulator builds on Apple Silicon Macs: aarch64-apple-ios-sim
    pub(crate) async fn verify_ios_tooling(&self, rustc: RustcDetails) -> Result<()> {
        // Check for xcrun and xcode-select
        let xcrun_output = tokio::process::Command::new("xcrun")
            .arg("--version")
            .output()
            .await;

        if let Err(e) = xcrun_output {
            tracing::warn!("xcrun not found: {e}. iOS builds may fail. Make sure Xcode and Xcode Command Line Tools are installed.");
        }

        // Determine which targets we need based on build configuration
        let mut required_targets = vec!["aarch64-apple-ios"]; // Always needed for device builds

        // For simulator builds, determine the appropriate target based on host architecture
        if self.build.target_args.device != Some(true) {
            // If a target was explicitly specified, use it
            if let Some(target) = self.build.target_args.target.as_deref() {
                required_targets.push(target);
            } else {
                // Otherwise, detect the host architecture and use the appropriate target
                match DioxusCrate::detect_host_arch() {
                    Some(arch) if arch == "x86_64" => {
                        required_targets.push("x86_64-apple-ios");
                    }
                    _ => {
                        // Default to aarch64-apple-ios-sim for Apple Silicon or unknown architectures
                        required_targets.push("aarch64-apple-ios-sim");
                    }
                }
            }
        }

        // Check if the required targets are installed
        for target in required_targets {
            if !rustc.has_target(target) {
                tracing::warn!("iOS target '{target}' is not installed. Run 'rustup target add {target}' to install it.");
            }
        }

        Ok(())
    }

    /// Check if the android tooling is installed
    ///
    /// looks for the android sdk + ndk
    ///
    /// will do its best to fill in the missing bits by exploring the sdk structure
    /// IE will attempt to use the Java installed from android studio if possible.
    pub(crate) async fn verify_android_tooling(&self, _rustc: RustcDetails) -> Result<()> {
        let result = self
            .krate
            .android_ndk()
            .map(|ndk| self.build.target_args.arch().android_linker(&ndk));

        if let Some(path) = result {
            if path.exists() {
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "Android linker not found. Please set the `ANDROID_NDK_HOME` environment variable to the root of your NDK installation."
        ).into())
    }

    /// Ensure the right dependencies are installed for linux apps.
    /// This varies by distro, so we just do nothing for now.
    ///
    /// Eventually, we want to check for the prereqs for wry/tao as outlined by tauri:
    ///     https://tauri.app/start/prerequisites/
    pub(crate) async fn verify_linux_tooling(&self, _rustc: RustcDetails) -> Result<()> {
        Ok(())
    }
}

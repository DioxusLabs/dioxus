use crate::{wasm_bindgen::WasmBindgen, BuildRequest, Error, Platform, Result, RustcDetails};
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

    /// Currently does nothing, but eventually we need to check that the mobile tooling is installed.
    ///
    /// For ios, this would be just aarch64-apple-ios + aarch64-apple-ios-sim, as well as xcrun and xcode-select
    ///
    /// We don't auto-install these yet since we're not doing an architecture check. We assume most users
    /// are running on an Apple Silicon Mac, but it would be confusing if we installed these when we actually
    /// should be installing the x86 versions.
    pub(crate) async fn verify_ios_tooling(&self, _rustc: RustcDetails) -> Result<()> {
        // open the simulator
        // _ = tokio::process::Command::new("open")
        //     .arg("/Applications/Xcode.app/Contents/Developer/Applications/Simulator.app")
        //     .stderr(Stdio::piped())
        //     .stdout(Stdio::piped())
        //     .status()
        //     .await;

        // Now xcrun to open the device
        // todo: we should try and query the device list and/or parse it rather than hardcode this simulator
        // _ = tokio::process::Command::new("xcrun")
        //     .args(["simctl", "boot", "83AE3067-987F-4F85-AE3D-7079EF48C967"])
        //     .stderr(Stdio::piped())
        //     .stdout(Stdio::piped())
        //     .status()
        //     .await;

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

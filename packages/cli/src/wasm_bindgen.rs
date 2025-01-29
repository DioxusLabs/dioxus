use crate::{CliSettings, Result};
use anyhow::{anyhow, Context};
use flate2::read::GzDecoder;
use std::path::PathBuf;
use std::{path::Path, process::Stdio};
use tar::Archive;
use tempfile::TempDir;
use tokio::{fs, process::Command};

pub(crate) struct WasmBindgen {
    version: String,
    input_path: PathBuf,
    out_dir: PathBuf,
    out_name: String,
    target: String,
    debug: bool,
    keep_debug: bool,
    demangle: bool,
    remove_name_section: bool,
    remove_producers_section: bool,
}

impl WasmBindgen {
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            input_path: PathBuf::new(),
            out_dir: PathBuf::new(),
            out_name: String::new(),
            target: String::new(),
            debug: true,
            keep_debug: true,
            demangle: true,
            remove_name_section: false,
            remove_producers_section: false,
        }
    }

    pub fn input_path(self, input_path: &Path) -> Self {
        Self {
            input_path: input_path.to_path_buf(),
            ..self
        }
    }

    pub fn out_dir(self, out_dir: &Path) -> Self {
        Self {
            out_dir: out_dir.to_path_buf(),
            ..self
        }
    }

    pub fn out_name(self, out_name: &str) -> Self {
        Self {
            out_name: out_name.to_string(),
            ..self
        }
    }

    pub fn target(self, target: &str) -> Self {
        Self {
            target: target.to_string(),
            ..self
        }
    }

    pub fn debug(self, debug: bool) -> Self {
        Self { debug, ..self }
    }

    pub fn keep_debug(self, keep_debug: bool) -> Self {
        Self { keep_debug, ..self }
    }

    pub fn demangle(self, demangle: bool) -> Self {
        Self { demangle, ..self }
    }

    pub fn remove_name_section(self, remove_name_section: bool) -> Self {
        Self {
            remove_name_section,
            ..self
        }
    }

    pub fn remove_producers_section(self, remove_producers_section: bool) -> Self {
        Self {
            remove_producers_section,
            ..self
        }
    }

    /// Run the bindgen command with the current settings
    pub async fn run(&self) -> Result<()> {
        let binary = self.get_binary_path().await?;

        let mut args = Vec::new();

        // Target
        args.push("--target");
        args.push(&self.target);

        // Options
        if self.debug {
            args.push("--debug");
        }

        if !self.demangle {
            args.push("--no-demangle");
        }

        if self.keep_debug {
            args.push("--keep-debug");
        }

        if self.remove_name_section {
            args.push("--remove-name-section");
        }

        if self.remove_producers_section {
            args.push("--remove-producers-section");
        }

        // Out name
        args.push("--out-name");
        args.push(&self.out_name);

        // wbg generates typescript bindnings by default - we don't want those
        args.push("--no-typescript");

        // Out dir
        let out_dir = self
            .out_dir
            .to_str()
            .expect("input_path should be valid utf8");

        args.push("--out-dir");
        args.push(out_dir);

        // Input path
        let input_path = self
            .input_path
            .to_str()
            .expect("input_path should be valid utf8");
        args.push(input_path);

        // Run bindgen
        Command::new(binary)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        Ok(())
    }

    /// Verify the installed version of wasm-bindgen-cli
    ///
    /// For local installations, this will check that the installed version matches the specified version.
    /// For managed installations, this will check that the version managed by `dx` is the specified version.
    pub async fn verify_install(version: &str) -> anyhow::Result<()> {
        let settings = Self::new(version);
        if CliSettings::prefer_no_downloads() {
            settings.verify_local_install().await
        } else {
            settings.verify_managed_install().await
        }
    }

    /// Install the specified wasm-bingen version.
    ///
    /// This will overwrite any existing wasm-bindgen binaries of the same version.
    ///
    /// This will attempt to install wasm-bindgen from:
    /// 1. Direct GitHub release download.
    /// 2. `cargo binstall` if installed.
    /// 3. Compile from source with `cargo install`.
    async fn install(&self) -> anyhow::Result<()> {
        tracing::info!("Installing wasm-bindgen-cli@{}...", self.version);

        // Attempt installation from GitHub
        if let Err(e) = self.install_github().await {
            tracing::error!("Failed to install wasm-bindgen-cli@{}: {e}", self.version);
        } else {
            tracing::info!(
                "wasm-bindgen-cli@{} was successfully installed from GitHub.",
                self.version
            );
            return Ok(());
        }

        // Attempt installation from binstall.
        if let Err(e) = self.install_binstall().await {
            tracing::error!("Failed to install wasm-bindgen-cli@{}: {e}", self.version);
            tracing::info!("Failed to install prebuilt binary for wasm-bindgen-cli@{}. Compiling from source instead. This may take a while.", self.version);
        } else {
            tracing::info!(
                "wasm-bindgen-cli@{} was successfully installed from cargo-binstall.",
                self.version
            );
            return Ok(());
        }

        // Attempt installation from cargo.
        self.install_cargo()
            .await
            .context("failed to install wasm-bindgen-cli from cargo")?;

        tracing::info!(
            "wasm-bindgen-cli@{} was successfully installed from source.",
            self.version
        );

        Ok(())
    }

    async fn install_github(&self) -> anyhow::Result<()> {
        tracing::debug!(
            "Attempting to install wasm-bindgen-cli@{} from GitHub",
            self.version
        );

        let url = self.git_install_url().ok_or_else(|| {
            anyhow!(
                "no available GitHub binary for wasm-bindgen-cli@{}",
                self.version
            )
        })?;

        // Get the final binary location.
        let binary_path = self.get_binary_path().await?;

        // Download then extract wasm-bindgen-cli.
        let bytes = reqwest::get(url).await?.bytes().await?;

        // Unpack the first tar entry to the final binary location
        Archive::new(GzDecoder::new(bytes.as_ref()))
            .entries()?
            .find(|entry| {
                entry
                    .as_ref()
                    .map(|e| {
                        e.path_bytes()
                            .ends_with(self.downloaded_bin_name().as_bytes())
                    })
                    .unwrap_or(false)
            })
            .context("Failed to find entry")??
            .unpack(&binary_path)
            .context("failed to unpack wasm-bindgen-cli binary")?;

        Ok(())
    }

    async fn install_binstall(&self) -> anyhow::Result<()> {
        tracing::debug!(
            "Attempting to install wasm-bindgen-cli@{} from cargo-binstall",
            self.version
        );

        let package = self.cargo_bin_name();
        let tempdir = TempDir::new()?;

        // Run install command
        Command::new("cargo")
            .args([
                "binstall",
                &package,
                "--no-confirm",
                "--force",
                "--no-track",
                "--install-path",
            ])
            .arg(tempdir.path())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await?;

        fs::copy(
            tempdir.path().join(self.downloaded_bin_name()),
            self.get_binary_path().await?,
        )
        .await?;

        Ok(())
    }

    async fn install_cargo(&self) -> anyhow::Result<()> {
        tracing::debug!(
            "Attempting to install wasm-bindgen-cli@{} from cargo-install",
            self.version
        );
        let package = self.cargo_bin_name();
        let tempdir = TempDir::new()?;

        // Run install command
        Command::new("cargo")
            .args([
                "install",
                &package,
                "--bin",
                "wasm-bindgen",
                "--no-track",
                "--force",
                "--root",
            ])
            .arg(tempdir.path())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .context("failed to install wasm-bindgen-cli from cargo-install")?;

        tracing::info!("Copying into path: {}", tempdir.path().display());

        // copy the wasm-bindgen out of the tempdir to the final location
        fs::copy(
            tempdir.path().join("bin").join(self.downloaded_bin_name()),
            self.get_binary_path().await?,
        )
        .await
        .context("failed to copy wasm-bindgen binary")?;

        Ok(())
    }

    async fn verify_local_install(&self) -> anyhow::Result<()> {
        tracing::trace!(
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

    async fn verify_managed_install(&self) -> anyhow::Result<()> {
        tracing::trace!(
            "Verifying wasm-bindgen-cli@{} is installed in the tool directory",
            self.version
        );

        let binary_name = self.installed_bin_name();
        let path = self.install_dir().await?.join(binary_name);

        if !path.exists() {
            self.install().await?;
        }

        Ok(())
    }

    async fn get_binary_path(&self) -> anyhow::Result<PathBuf> {
        if CliSettings::prefer_no_downloads() {
            which::which("wasm-bindgen")
                .map_err(|_| anyhow!("Missing wasm-bindgen-cli@{}", self.version))
        } else {
            let installed_name = self.installed_bin_name();
            let install_dir = self.install_dir().await?;
            Ok(install_dir.join(installed_name))
        }
    }

    async fn install_dir(&self) -> anyhow::Result<PathBuf> {
        let bindgen_dir = dirs::data_local_dir()
            .expect("user should be running on a compatible operating system")
            .join("dioxus/wasm-bindgen/");

        fs::create_dir_all(&bindgen_dir).await?;
        Ok(bindgen_dir)
    }

    fn installed_bin_name(&self) -> String {
        let mut name = format!("wasm-bindgen-{}", self.version);
        if cfg!(windows) {
            name = format!("{name}.exe");
        }
        name
    }

    fn cargo_bin_name(&self) -> String {
        format!("wasm-bindgen-cli@{}", self.version)
    }

    fn downloaded_bin_name(&self) -> &'static str {
        if cfg!(windows) {
            "wasm-bindgen.exe"
        } else {
            "wasm-bindgen"
        }
    }

    fn git_install_url(&self) -> Option<String> {
        let platform = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            "x86_64-pc-windows-msvc"
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            "x86_64-unknown-linux-musl"
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            "aarch64-unknown-linux-gnu"
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            "x86_64-apple-darwin"
        } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            "aarch64-apple-darwin"
        } else {
            return None;
        };

        Some(format!(
            "https://github.com/rustwasm/wasm-bindgen/releases/download/{}/wasm-bindgen-{}-{}.tar.gz",
            self.version, self.version, platform
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const VERSION: &str = "0.2.99";

    /// Test the github installer.
    #[tokio::test]
    async fn test_github_install() {
        let binary = WasmBindgen::new(VERSION);
        reset_test().await;
        binary.install_github().await.unwrap();
        test_verify_install().await;
        verify_installation(&binary).await;
    }

    /// Test the cargo installer.
    #[tokio::test]
    async fn test_cargo_install() {
        let binary = WasmBindgen::new(VERSION);
        reset_test().await;
        binary.install_cargo().await.unwrap();
        test_verify_install().await;
        verify_installation(&binary).await;
    }

    // CI doesn't have binstall.
    // Test the binstall installer
    // #[tokio::test]
    // async fn test_binstall_install() {
    //     let binary = WasmBindgen::new(VERSION);
    //     reset_test().await;
    //     binary.install_binstall().await.unwrap();
    //     test_verify_install().await;
    //     verify_installation(&binary).await;
    // }

    /// Helper to test `verify_install` after an installation.
    async fn test_verify_install() {
        WasmBindgen::verify_install(VERSION).await.unwrap();
    }

    /// Helper to test that the installed binary actually exists.
    async fn verify_installation(binary: &WasmBindgen) {
        let path = binary.install_dir().await.unwrap();
        let name = binary.installed_bin_name();
        let binary_path = path.join(name);
        assert!(
            binary_path.exists(),
            "wasm-bindgen binary doesn't exist after installation"
        );
    }

    /// Delete the installed binary. The temp folder should be automatically deleted.
    async fn reset_test() {
        let binary = WasmBindgen::new(VERSION);
        let path = binary.install_dir().await.unwrap();
        let name = binary.installed_bin_name();
        let binary_path = path.join(name);
        let _ = tokio::fs::remove_file(binary_path).await;
    }
}

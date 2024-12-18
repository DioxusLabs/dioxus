use anyhow::{anyhow, Context};
use flate2::read::GzDecoder;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
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
    pub async fn run(&self) -> anyhow::Result<()> {
        let binary = Self::final_binary(&self.version).await?;

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

        // Out dir
        let canonical_out_dir = self
            .out_dir
            .canonicalize()
            .expect("out_dir should resolve to a valid path");

        let out_dir = canonical_out_dir
            .to_str()
            .expect("out_dir should be valid UTF-8");

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

    /// Verify that the required wasm-bindgen version is installed.
    pub async fn verify_install(version: &str) -> anyhow::Result<bool> {
        let binary_name = Self::installed_bin_name(version);
        let path = Self::install_dir().await?.join(binary_name);
        Ok(path.exists())
    }

    /// Install the specified wasm-bingen version.
    ///
    /// This will overwrite any existing wasm-bindgen binaries of the same version.
    ///
    /// This will attempt to install wasm-bindgen from:
    /// 1. Direct GitHub release download.
    /// 2. `cargo binstall` if installed.
    /// 3. Compile from source with `cargo install`.
    pub async fn install(version: &str) -> anyhow::Result<()> {
        tracing::info!("Installing wasm-bindgen-cli@{version}...");

        // Attempt installation from GitHub
        if let Err(e) = Self::install_github(version).await {
            tracing::error!("Failed to install wasm-bindgen-cli@{version}: {e}");
        } else {
            tracing::info!("wasm-bindgen-cli@{version} was successfully installed from GitHub.");
            return Ok(());
        }

        // Attempt installation from binstall.
        if let Err(e) = Self::install_binstall(version).await {
            tracing::error!("Failed to install wasm-bindgen-cli@{version}: {e}");
            tracing::info!("Failed to install prebuilt binary for wasm-bindgen-cli@{version}. Compiling from source instead. This may take a while.");
        } else {
            tracing::info!(
                "wasm-bindgen-cli@{version} was successfully installed from cargo-binstall."
            );
            return Ok(());
        }

        // Attempt installation from cargo.
        Self::install_cargo(version)
            .await
            .context("failed to install wasm-bindgen-cli from cargo")?;

        tracing::info!("wasm-bindgen-cli@{version} was successfully installed from source.");

        Ok(())
    }

    /// Try installing wasm-bindgen-cli from GitHub.
    async fn install_github(version: &str) -> anyhow::Result<()> {
        tracing::debug!("Attempting to install wasm-bindgen-cli@{version} from GitHub");

        let url = git_install_url(version)
            .ok_or_else(|| anyhow!("no available GitHub binary for wasm-bindgen-cli@{version}"))?;

        // Get the final binary location.
        let final_binary = Self::final_binary(version).await?;

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
                            .ends_with(Self::downloaded_bin_name().as_bytes())
                    })
                    .unwrap_or(false)
            })
            .context("Failed to find entry")??
            .unpack(&final_binary)
            .context("failed to unpack wasm-bindgen-cli binary")?;

        Ok(())
    }

    /// Try installing wasm-bindgen-cli through cargo-binstall.
    async fn install_binstall(version: &str) -> anyhow::Result<()> {
        tracing::debug!("Attempting to install wasm-bindgen-cli@{version} from cargo-binstall");

        let package = Self::cargo_bin_name(version);
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        fs::copy(
            tempdir.path().join(Self::downloaded_bin_name()),
            Self::final_binary(version).await?,
        )
        .await?;

        Ok(())
    }

    /// Try installing wasm-bindgen-cli from source using cargo install.
    async fn install_cargo(version: &str) -> anyhow::Result<()> {
        tracing::debug!("Attempting to install wasm-bindgen-cli@{version} from cargo-install");
        let package = Self::cargo_bin_name(version);
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("failed to install wasm-bindgen-cli from cargo-install")?;

        tracing::info!("Copying into path: {}", tempdir.path().display());

        // copy the wasm-bindgen out of the tempdir to the final location
        fs::copy(
            tempdir.path().join("bin").join(Self::downloaded_bin_name()),
            Self::final_binary(version).await?,
        )
        .await
        .context("failed to copy wasm-bindgen binary")?;

        Ok(())
    }

    /// Get the installation directory for the wasm-bindgen executable.
    async fn install_dir() -> anyhow::Result<PathBuf> {
        let bindgen_dir = dirs::data_local_dir()
            .expect("user should be running on a compatible operating system")
            .join("dioxus/wasm-bindgen/");

        fs::create_dir_all(&bindgen_dir).await?;

        Ok(bindgen_dir)
    }

    /// Get the name of a potentially installed wasm-bindgen binary.
    fn installed_bin_name(version: &str) -> String {
        let mut name = format!("wasm-bindgen-{version}");
        if cfg!(windows) {
            name = format!("{name}.exe");
        }
        name
    }

    /// Get the crates.io package name of wasm-bindgen-cli.
    fn cargo_bin_name(version: &str) -> String {
        format!("wasm-bindgen-cli@{version}")
    }

    async fn final_binary(version: &str) -> Result<PathBuf, anyhow::Error> {
        let installed_name = Self::installed_bin_name(version);
        let install_dir = Self::install_dir().await?;
        Ok(install_dir.join(installed_name))
    }

    fn downloaded_bin_name() -> &'static str {
        if cfg!(windows) {
            "wasm-bindgen.exe"
        } else {
            "wasm-bindgen"
        }
    }
}

/// Get the GitHub installation URL for wasm-bindgen if it exists.
fn git_install_url(version: &str) -> Option<String> {
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

    Some(format!("https://github.com/rustwasm/wasm-bindgen/releases/download/{version}/wasm-bindgen-{version}-{platform}.tar.gz"))
}

/// A builder for WasmBindgen options.
pub(crate) struct WasmBindgenBuilder {
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

impl WasmBindgenBuilder {
    pub fn new(version: String) -> Self {
        Self {
            version,
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

    pub fn build(self) -> WasmBindgen {
        WasmBindgen {
            version: self.version,
            input_path: self.input_path,
            out_dir: self.out_dir,
            out_name: self.out_name,
            target: self.target,
            debug: self.debug,
            keep_debug: self.keep_debug,
            demangle: self.demangle,
            remove_name_section: self.remove_name_section,
            remove_producers_section: self.remove_producers_section,
        }
    }

    pub fn input_path(self, input_path: &Path) -> Self {
        Self {
            input_path: input_path.to_path_buf(),
            ..self
        }
    }

    pub fn out_dir(mut self, out_dir: &Path) -> Self {
        self.out_dir = out_dir.canonicalize().expect("Invalid out_dir path");
        self
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
}

#[cfg(test)]
mod test {
    use super::*;
    const VERSION: &str = "0.2.99";

    /// Test the github installer.
    #[tokio::test]
    async fn test_github_install() {
        reset_test().await;
        WasmBindgen::install_github(VERSION).await.unwrap();
        test_verify_install().await;
        verify_installation().await;
    }

    /// Test the cargo installer.
    #[tokio::test]
    async fn test_cargo_install() {
        reset_test().await;
        WasmBindgen::install_cargo(VERSION).await.unwrap();
        test_verify_install().await;
        verify_installation().await;
    }

    // CI doesn't have binstall.
    // Test the binstall installer
    // #[tokio::test]
    // async fn test_binstall_install() {
    //     reset_test().await;
    //     WasmBindgen::install_binstall(VERSION).await.unwrap();
    //     test_verify_install().await;
    //     verify_installation().await;
    // }

    /// Helper to test `WasmBindgen::verify_install` after an installation.
    async fn test_verify_install() {
        // Test install verification
        let is_installed = WasmBindgen::verify_install(VERSION).await.unwrap();
        assert!(
            is_installed,
            "wasm-bingen install verification returned false after installation"
        );
    }

    /// Helper to test that the installed binary actually exists.
    async fn verify_installation() {
        let path = WasmBindgen::install_dir().await.unwrap();
        let name = WasmBindgen::installed_bin_name(VERSION);
        let binary = path.join(name);
        assert!(
            binary.exists(),
            "wasm-bindgen binary doesn't exist after installation"
        );
    }

    /// Delete the installed binary. The temp folder should be automatically deleted.
    async fn reset_test() {
        let path = WasmBindgen::install_dir().await.unwrap();
        let name = WasmBindgen::installed_bin_name(VERSION);
        let binary = path.join(name);
        let _ = fs::remove_file(binary).await;
    }
}

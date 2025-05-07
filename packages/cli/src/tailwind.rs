use crate::{CliSettings, Result};
use anyhow::{anyhow, Context};
use flate2::read::GzDecoder;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tar::Archive;
use tempfile::TempDir;
use tokio::{fs, process::Command};

pub(crate) struct TailwindCli {
    version: String,
}

impl TailwindCli {
    const VERSION: &'static str = "v4.1.5";

    pub(crate) fn new(version: String) -> Self {
        Self { version }
    }

    pub(crate) async fn watch(
        &self,
        input_path: &PathBuf,
        output_path: &PathBuf,
    ) -> Result<tokio::process::Child> {
        let binary_path = self.get_binary_path().await?;
        let mut cmd = Command::new(binary_path);
        let proc = cmd
            .arg("--watch")
            .arg(input_path)
            .arg("--output")
            .arg(output_path)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(proc)
    }
    pub(crate) fn run(&self) {}

    async fn get_binary_path(&self) -> anyhow::Result<PathBuf> {
        if CliSettings::prefer_no_downloads() {
            which::which("tailwindcss")
                .map_err(|_| anyhow!("Missing wasm-bindgen-cli@{}", self.version))
        } else {
            let installed_name = self.installed_bin_name();
            let install_dir = self.install_dir().await?;
            Ok(install_dir.join(installed_name))
        }
    }

    fn installed_bin_name(&self) -> String {
        let mut name = format!("tailwindcss-{}", self.version);
        if cfg!(windows) {
            name = format!("{name}.exe");
        }
        name
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

    fn downloaded_bin_name(&self) -> &'static str {
        if cfg!(windows) {
            "tailwindcss.exe"
        } else {
            "tailwindcss"
        }
    }

    async fn install_dir(&self) -> anyhow::Result<PathBuf> {
        let bindgen_dir = dirs::data_local_dir()
            .expect("user should be running on a compatible operating system")
            .join("dioxus/wasm-bindgen/");

        fs::create_dir_all(&bindgen_dir).await?;
        Ok(bindgen_dir)
    }

    fn git_install_url(&self) -> Option<String> {
        let platform = match target_lexicon::HOST.operating_system {
            target_lexicon::OperatingSystem::Linux => "linux",
            target_lexicon::OperatingSystem::Darwin(_) => "macos",
            target_lexicon::OperatingSystem::Windows => "windows",
            _ => return None,
        };

        let arch = match target_lexicon::HOST.architecture {
            target_lexicon::Architecture::X86_64 if platform == "windows" => "x64.exe",
            target_lexicon::Architecture::X86_64 => "x64",
            target_lexicon::Architecture::Aarch64(_) => "arm64",
            _ => return None,
        };

        // eg:
        //
        // https://github.com/tailwindlabs/tailwindcss/releases/download/v4.1.5/tailwindcss-linux-arm64
        //
        // tailwindcss-linux-arm64
        // tailwindcss-linux-x64
        // tailwindcss-macos-arm64
        // tailwindcss-macos-x64
        // tailwindcss-windows-x64.exe
        // tailwindcss-linux-arm64-musl
        // tailwindcss-linux-x64-musl
        Some(format!(
            "https://github.com/tailwindlabs/tailwindcss/releases/download/{}/tailwind-{}-{}.tar.gz",
            self.version, platform, arch
        ))
    }
}

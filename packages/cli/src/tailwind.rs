use crate::{CliSettings, Result, Workspace};
use anyhow::{anyhow, Context};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

#[derive(Debug)]
pub(crate) struct TailwindCli {
    version: String,
}

impl TailwindCli {
    const V3_TAG: &'static str = "v3.4.15";
    const V4_TAG: &'static str = "v4.1.5";

    pub(crate) fn new(version: String) -> Self {
        Self { version }
    }

    pub(crate) async fn run_once(
        manifest_dir: PathBuf,
        input_path: Option<PathBuf>,
        output_path: Option<PathBuf>,
    ) -> Result<()> {
        let Some(tailwind) = Self::autodetect(&manifest_dir, &input_path) else {
            return Ok(());
        };

        if !tailwind.get_binary_path()?.exists() {
            tracing::info!("Installing tailwindcss@{}", tailwind.version);
            tailwind.install_github().await?;
        }

        let output = tailwind
            .run(&manifest_dir, input_path, output_path, false)?
            .wait_with_output()
            .await?;

        if !output.stderr.is_empty() {
            tracing::warn!(
                "Warnings while running tailwind: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }

        Ok(())
    }

    pub(crate) fn serve(
        manifest_dir: PathBuf,
        input_path: Option<PathBuf>,
        output_path: Option<PathBuf>,
    ) -> tokio::task::JoinHandle<Result<()>> {
        tokio::spawn(async move {
            let Some(tailwind) = Self::autodetect(&manifest_dir, &input_path) else {
                return Ok(());
            };

            if !tailwind.get_binary_path()?.exists() {
                tracing::info!("Installing tailwindcss@{}", tailwind.version);
                tailwind.install_github().await?;
            }

            // the tw watcher blocks on stdin, and `.wait()` will drop stdin
            // unfortunately the tw watcher just deadlocks in this case, so we take the stdin manually
            let mut proc = tailwind.run(&manifest_dir, input_path, output_path, true)?;
            let stdin = proc.stdin.take();
            proc.wait().await?;
            drop(stdin);

            Ok(())
        })
    }

    /// Use the correct tailwind version based on the manifest directory.
    ///
    /// - If `tailwind.config.js` or `tailwind.config.ts` exists, use v3.
    /// - If `tailwind.css` exists, use v4.
    ///
    /// Note that v3 still uses the tailwind.css file, but usually the accompanying js file indicates
    /// that the project is using v3.
    pub(crate) fn autodetect(manifest_dir: &Path, input_path: &Option<PathBuf>) -> Option<Self> {
        let dir = input_path
            .as_ref()
            .map(|p| manifest_dir.join(p))
            .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
            .unwrap_or(manifest_dir.to_path_buf());

        if dir.join("tailwind.config.js").exists() || dir.join("tailwind.config.ts").exists() {
            return Some(Self::v3());
        }

        if input_path
            .as_ref()
            .map(|p| manifest_dir.join(p).exists())
            .unwrap_or_else(|| manifest_dir.join("tailwind.css").exists())
        {
            return Some(Self::v4());
        }

        None
    }

    pub(crate) fn v4() -> Self {
        Self::new(Self::V4_TAG.to_string())
    }

    pub(crate) fn v3() -> Self {
        Self::new(Self::V3_TAG.to_string())
    }

    pub(crate) fn run(
        &self,
        manifest_dir: &Path,
        input_path: Option<PathBuf>,
        output_path: Option<PathBuf>,
        watch: bool,
    ) -> Result<tokio::process::Child> {
        let binary_path = self.get_binary_path()?;

        let input_path = input_path.unwrap_or_else(|| manifest_dir.join("tailwind.css"));
        let output_path =
            output_path.unwrap_or_else(|| manifest_dir.join("assets").join("tailwind.css"));

        if !output_path.exists() {
            std::fs::create_dir_all(output_path.parent().unwrap())
                .context("failed to create tailwindcss output directory")?;
        }

        tracing::debug!("Spawning tailwindcss@{} with args: {:?}", self.version, {
            [
                binary_path.to_string_lossy().to_string(),
                "--input".to_string(),
                input_path.to_string_lossy().to_string(),
                "--output".to_string(),
                output_path.to_string_lossy().to_string(),
                "--watch".to_string(),
            ]
        });

        let mut cmd = Command::new(binary_path);
        let proc = cmd
            .arg("--input")
            .arg(input_path)
            .arg("--output")
            .arg(output_path)
            .args(watch.then_some("--watch"))
            .current_dir(manifest_dir)
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        Ok(proc)
    }

    pub fn get_binary_path(&self) -> anyhow::Result<PathBuf> {
        if CliSettings::prefer_no_downloads() {
            which::which("tailwindcss").map_err(|_| anyhow!("Missing tailwindcss@{}", self.version))
        } else {
            let installed_name = self.installed_bin_name();
            let install_dir = self.install_dir()?;
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
            "Attempting to install tailwindcss@{} from GitHub",
            self.version
        );

        let url = self.git_install_url().ok_or_else(|| {
            anyhow!(
                "no available GitHub binary for tailwindcss@{}",
                self.version
            )
        })?;

        // Get the final binary location.
        let binary_path = self.get_binary_path()?;

        // Download then extract tailwindcss.
        let bytes = reqwest::get(url).await?.bytes().await?;

        std::fs::create_dir_all(binary_path.parent().unwrap())
            .context("failed to create tailwindcss directory")?;

        std::fs::write(&binary_path, &bytes).context("failed to write tailwindcss binary")?;

        // Make the binary executable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = binary_path.metadata()?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary_path, perms)?;
        }

        Ok(())
    }

    fn downloaded_bin_name(&self) -> Option<String> {
        let platform = match target_lexicon::HOST.operating_system {
            target_lexicon::OperatingSystem::Linux => "linux",
            target_lexicon::OperatingSystem::Darwin(_) => "macos",
            target_lexicon::OperatingSystem::Windows => "windows",
            _ => return None,
        };

        let arch = match target_lexicon::HOST.architecture {
            target_lexicon::Architecture::X86_64 if platform == "windows" => "x64.exe",
            target_lexicon::Architecture::X86_64 => "x64",
            // you would think this would be arm64.exe, but tailwind doesn't distribute arm64 binaries
            target_lexicon::Architecture::Aarch64(_) if platform == "windows" => "x64.exe",
            target_lexicon::Architecture::Aarch64(_) => "arm64",
            _ => return None,
        };

        Some(format!("tailwindcss-{platform}-{arch}"))
    }

    fn install_dir(&self) -> Result<PathBuf> {
        let bindgen_dir = Workspace::dioxus_data_dir().join("tailwind/");
        Ok(bindgen_dir)
    }

    fn git_install_url(&self) -> Option<String> {
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
            "https://github.com/tailwindlabs/tailwindcss/releases/download/{}/{}",
            self.version,
            self.downloaded_bin_name()?
        ))
    }
}

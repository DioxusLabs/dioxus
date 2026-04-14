use super::*;
use crate::{Result, Workspace};
use anyhow::{bail, Context};
use itertools::Itertools;
use self_update::cargo_crate_version;

/// Run the project with the given arguments
///
/// This is a shorthand for `dx serve` with interactive mode and hot-reload disabled.
#[derive(Clone, Debug, Parser)]
pub(crate) struct SelfUpdate {
    /// Use the latest nightly build.
    #[clap(long, default_value = "false")]
    pub nightly: bool,

    /// Specify a version to install.
    #[clap(long)]
    pub version: Option<String>,

    /// Install the update.
    #[clap(long, default_value = "true", num_args = 0..=1)]
    pub install: bool,

    /// List available versions.
    #[clap(long, default_value = "false")]
    pub list: bool,

    /// Force the update even if the current version is up to date.
    #[clap(long, default_value = "false")]
    pub force: bool,
}

impl SelfUpdate {
    pub async fn self_update(self) -> Result<StructuredOutput> {
        tokio::task::spawn_blocking(move || {
            let start = std::time::Instant::now();
            if self.list {
                let res = self_update::backends::github::Update::configure()
                    .repo_owner("dioxuslabs")
                    .repo_name("dioxus")
                    .bin_name("dx")
                    .current_version(cargo_crate_version!())
                    .build()
                    .unwrap()
                    .get_latest_releases(cargo_crate_version!())
                    .context("Failed to fetch latest version")?;

                if res.is_empty() {
                    tracing::info!("Your version {} is up to date!", cargo_crate_version!());
                } else {
                    tracing::info!("Your version {} is out of date!", cargo_crate_version!());
                    tracing::info!(
                        "Available versions: [{}]",
                        res.iter()
                            .map(|r| r.version.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }

                return Ok(StructuredOutput::Success);
            }

            let repo = self_update::backends::github::Update::configure()
                .repo_owner("dioxuslabs")
                .repo_name("dioxus")
                .bin_name("dx")
                .current_version(cargo_crate_version!())
                .build()
                .unwrap();

            let force = self.force || self.version.is_some();
            let latest = match self.version {
                Some(version) => repo
                    .get_release_version(&version)
                    .context("Failed to fetch release by tag")?,
                None => repo
                    .get_latest_release()
                    .context("Failed to fetch latest version")?,
            };

            if latest.version == cargo_crate_version!() && !force {
                tracing::info!("Your version {} is up to date!", cargo_crate_version!());
                return Ok(StructuredOutput::Success);
            }

            tracing::info!("Your version is out of date!");
            tracing::info!("- Yours:  {}", cargo_crate_version!());
            tracing::info!("- Latest: {}", latest.version);

            let cur_arch = if cfg!(target_arch = "x86_64") {
                "x86_64"
            } else if cfg!(target_arch = "aarch64") {
                "aarch64"
            } else {
                bail!("Unsupported architecture");
            };

            let cur_os = if cfg!(target_os = "windows") {
                "windows"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else if cfg!(target_os = "macos") {
                "darwin"
            } else {
                bail!("Unsupported OS");
            };

            let zip_ext = "zip";

            tracing::debug!("Available assets: {:?}", latest.assets);

            let asset = latest
                .assets
                .iter()
                .find(|a| {
                    a.name.contains(cur_os)
                        && a.name.contains(cur_arch)
                        && a.name.ends_with(zip_ext)
                })
                .context("No suitable asset found")?;

            let install_dir = Workspace::dioxus_data_dir().join("self-update");
            std::fs::create_dir_all(&install_dir).context("Failed to create install directory")?;

            tracing::info!("Downloading update from Github");
            tracing::debug!("Download URL: {}", asset.download_url);
            let body = latest.body.unwrap_or_default();
            let brief = vec![
                latest.name.to_string(),
                "".to_string(),
                latest.date.to_string(),
                asset.download_url.to_string(),
                "".to_string(),
            ]
            .into_iter()
            .chain(body.lines().map(ToString::to_string).take(7))
            .chain(std::iter::once(" ...".to_string()))
            .map(|line| format!("                | {line}"))
            .join("\n");

            tracing::info!("{}", brief.trim());

            let archive_path = install_dir.join(&asset.name);
            _ = std::fs::remove_file(&archive_path).ok();
            let archive_file = std::fs::File::create(&archive_path)?;
            let download_url = asset.download_url.clone();
            self_update::Download::from_url(&download_url)
                .set_header(
                    hyper::http::header::ACCEPT,
                    "application/octet-stream".parse().unwrap(),
                )
                .download_to(archive_file)
                .context("Failed to download update")?;

            let install_dir = install_dir.join("dx");
            _ = std::fs::remove_dir_all(&install_dir);
            self_update::Extract::from_source(&archive_path)
                .extract_into(&install_dir)
                .context("Failed to extract update")?;

            let exe = if cfg!(target_os = "windows") {
                "dx.exe"
            } else {
                "dx"
            };
            let executable = install_dir.join(exe);
            if !executable.exists() {
                bail!("Executable not found in {}", install_dir.display());
            }

            tracing::info!(
                "Successfully downloaded update in {}ms! 👍",
                start.elapsed().as_millis()
            );

            if self.install {
                tracing::info!(
                    "Installing dx v{} to {}",
                    latest.version,
                    std::env::current_exe()?.display()
                );

                if !self.force {
                    tracing::warn!("Continue? (y/n)");
                    print!("                > ");
                    std::io::stdout()
                        .flush()
                        .context("Failed to flush stdout")?;
                    let mut input = String::new();
                    std::io::stdin()
                        .read_line(&mut input)
                        .context("Failed to read input")?;
                    if !input.trim().to_ascii_lowercase().starts_with('y') {
                        tracing::info!("Aborting update");
                        return Ok(StructuredOutput::Success);
                    }
                }

                self_update::self_replace::self_replace(executable)?;
                let time_taken = start.elapsed().as_millis();
                tracing::info!("Done in {} ms! 💫", time_taken)
            } else {
                tracing::info!("Update downloaded to {}", install_dir.display());
                tracing::info!("Run `dx self-update --install` to install the update");
            }

            Ok(StructuredOutput::Success)
        })
        .await
        .context("Failed to run self-update")?
    }
}

/// Check against the github release list to see if the currently released `dx` version is
/// more up-to-date than our own.
///
/// We only toss out this warning once and then save to the settings file to ignore this version
/// in the future.
pub fn log_if_cli_could_update() {
    tokio::task::spawn_blocking(|| {
        let release = self_update::backends::github::Update::configure()
            .repo_owner("dioxuslabs")
            .repo_name("dioxus")
            .bin_name("dx")
            .current_version(cargo_crate_version!())
            .build()
            .unwrap()
            .get_latest_release();

        if let Ok(release) = release {
            let old = krates::semver::Version::parse(cargo_crate_version!());
            let new = krates::semver::Version::parse(&release.version);

            if let (Ok(old), Ok(new)) = (old, new) {
                if old < new {
                    _ = crate::CliSettings::modify_settings(|f| {
                        let ignored = f.ignore_version_update.as_deref().unwrap_or_default();
                        if release.version != ignored {
                            use crate::styles::GLOW_STYLE;
                            tracing::warn!("A new dx version is available: {new}! Run {GLOW_STYLE}dx self-update{GLOW_STYLE:#} to update.");
                            f.ignore_version_update = Some(new.to_string());
                        }
                    });
                }
            }
        }
    });
}

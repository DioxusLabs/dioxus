use crate::cli::serve::ServeArguments;
use crate::config::Platform;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use crate::{build::Build, TraceSrc};
use futures_util::stream::select_all;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::str::FromStr;
use std::{path::PathBuf, process::Stdio};
use tokio::process::{Child, Command};

mod cargo;
mod fullstack;
mod prepare_html;
mod progress;
mod web;
pub use progress::{Stage, UpdateBuildProgress, UpdateStage};

/// The target platform for the build
/// This is very similar to the Platform enum, but we need to be able to differentiate between the
/// server and web targets for the fullstack platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetPlatform {
    Web,
    Desktop,
    Server,
    Liveview,
}

impl FromStr for TargetPlatform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "web" => Ok(Self::Web),
            "desktop" => Ok(Self::Desktop),
            "axum" | "server" => Ok(Self::Server),
            "liveview" => Ok(Self::Liveview),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for TargetPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetPlatform::Web => write!(f, "web"),
            TargetPlatform::Desktop => write!(f, "desktop"),
            TargetPlatform::Server => write!(f, "server"),
            TargetPlatform::Liveview => write!(f, "liveview"),
        }
    }
}

/// A request for a project to be built
#[derive(Clone)]
pub struct BuildRequest {
    /// Whether the build is for serving the application
    pub serve: bool,
    /// The configuration for the crate we are building
    pub dioxus_crate: DioxusCrate,
    /// The target platform for the build
    pub target_platform: TargetPlatform,
    /// The arguments for the build
    pub build_arguments: Build,
    /// The rustc flags to pass to the build
    pub rust_flags: Vec<String>,
    /// The target directory for the build
    pub target_dir: Option<PathBuf>,
}

impl BuildRequest {
    pub fn create(
        serve: bool,
        dioxus_crate: &DioxusCrate,
        build_arguments: impl Into<Build>,
    ) -> crate::Result<Vec<Self>> {
        let build_arguments = build_arguments.into();
        let platform = build_arguments.platform();
        let single_platform = |platform| {
            let dioxus_crate = dioxus_crate.clone();
            vec![Self {
                serve,
                dioxus_crate,
                build_arguments: build_arguments.clone(),
                target_platform: platform,
                rust_flags: Default::default(),
                target_dir: Default::default(),
            }]
        };
        Ok(match platform {
            Platform::Liveview => single_platform(TargetPlatform::Liveview),
            Platform::Web => single_platform(TargetPlatform::Web),
            Platform::Desktop => single_platform(TargetPlatform::Desktop),
            Platform::StaticGeneration | Platform::Fullstack => {
                Self::new_fullstack(dioxus_crate.clone(), build_arguments, serve)?
            }
        })
    }

    pub(crate) async fn build_all_parallel(
        build_requests: Vec<BuildRequest>,
    ) -> Result<Vec<BuildResult>> {
        let multi_platform_build = build_requests.len() > 1;
        let mut build_progress = Vec::new();
        let mut set = tokio::task::JoinSet::new();
        for build_request in build_requests {
            let (tx, rx) = futures_channel::mpsc::unbounded();
            build_progress.push((build_request.build_arguments.platform(), rx));
            set.spawn(async move { build_request.build(tx).await });
        }

        // Watch the build progress as it comes in
        loop {
            let mut next = select_all(
                build_progress
                    .iter_mut()
                    .map(|(platform, rx)| rx.map(move |update| (*platform, update))),
            );
            match next.next().await {
                Some((platform, update)) => {
                    if multi_platform_build {
                        print!("{platform} build: ");
                        update.to_std_out();
                    } else {
                        update.to_std_out();
                    }
                }
                None => {
                    break;
                }
            }
        }

        let mut all_results = Vec::new();

        while let Some(result) = set.join_next().await {
            let result = result
                .map_err(|_| crate::Error::Unique("Failed to build project".to_owned()))??;
            all_results.push(result);
        }

        Ok(all_results)
    }

    /// Check if the build is targeting the web platform
    pub fn targeting_web(&self) -> bool {
        self.target_platform == TargetPlatform::Web
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BuildResult {
    pub executable: PathBuf,
    pub target_platform: TargetPlatform,
}

impl BuildResult {
    /// Open the executable if this is a native build
    pub fn open(
        &self,
        serve: &ServeArguments,
        fullstack_address: Option<SocketAddr>,
        workspace: &std::path::Path,
        asset_root: &std::path::Path,
        devserver_addr: SocketAddr,
        app_title: String,
    ) -> std::io::Result<Option<Child>> {
        match self.target_platform {
            TargetPlatform::Web => {
                tracing::info!(dx_src = ?TraceSrc::Dev, "Serving web app on http://{} 🎉", serve.address.address());
                return Ok(None);
            }
            TargetPlatform::Desktop => {
                tracing::info!(dx_src = ?TraceSrc::Dev, "Launching desktop app at {} 🎉", self.executable.display());
            }
            TargetPlatform::Server => {
                // shut this up for now - the web app will take priority
            }
            TargetPlatform::Liveview => {
                if let Some(fullstack_address) = fullstack_address {
                    tracing::info!(
                        dx_src = ?TraceSrc::Dev,
                        "Launching liveview server on http://{:?} 🎉",
                        fullstack_address
                    );
                }
            }
        }

        tracing::info!(dx_src = ?TraceSrc::Dev, "Press [o] to open the app manually.");

        let executable = self.executable.canonicalize()?;
        let mut cmd = Command::new(executable);

        // Set the env vars that the clients will expect
        // These need to be stable within a release version (ie 0.6.0)
        cmd.env(dioxus_cli_config::CLI_ENABLED_ENV, "true");
        if let Some(addr) = fullstack_address {
            cmd.env(dioxus_cli_config::SERVER_IP_ENV, addr.ip().to_string());
            cmd.env(dioxus_cli_config::SERVER_PORT_ENV, addr.port().to_string());
        }
        cmd.env(
            dioxus_cli_config::ALWAYS_ON_TOP_ENV,
            serve.always_on_top.unwrap_or(true).to_string(),
        );
        cmd.env(
            dioxus_cli_config::ASSET_ROOT_ENV,
            asset_root.display().to_string(),
        );
        cmd.env(
            dioxus_cli_config::DEVSERVER_RAW_ADDR_ENV,
            devserver_addr.to_string(),
        );
        cmd.env(dioxus_cli_config::APP_TITLE_ENV, app_title);

        cmd.stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .current_dir(workspace);

        Ok(Some(cmd.spawn()?))
    }
}

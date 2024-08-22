use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use crate::{build::Build, config};
use crate::{cli::serve::ServeArguments, config::Platform};
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
pub use progress::{
    BuildMessage, MessageSource, MessageType, Stage, UpdateBuildProgress, UpdateStage,
};

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
            _ => unimplemented!("Unknown platform: {platform:?}"),
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
        config: &DioxusCrate,
        serve: &ServeArguments,
        fullstack_address: Option<SocketAddr>,
        workspace: &std::path::Path,
    ) -> std::io::Result<Option<Child>> {
        if self.target_platform == TargetPlatform::Web {
            return Ok(None);
        }
        if self.target_platform == TargetPlatform::Server {
            tracing::trace!("Proxying fullstack server from port {fullstack_address:?}");
        }

        todo!("set runtime env vars for the fullstack server")
        // let arguments = RuntimeCLIArguments::new(serve.address.address(), fullstack_address);
        // let executable = self.executable.canonicalize()?;
        // let mut cmd = Command::new(executable);
        // cmd
        //     // When building the fullstack server, we need to forward the serve arguments (like port) to the fullstack server through env vars
        //     // .env(
        //     //     dioxus_cli_config::__private::SERVE_ENV,
        //     //     serde_json::to_string(&arguments).unwrap(),
        //     // )
        //     .env("CARGO_MANIFEST_DIR", config.crate_dir())
        //     .stderr(Stdio::piped())
        //     .stdout(Stdio::piped())
        //     .kill_on_drop(true)
        //     .current_dir(workspace);
        // Ok(Some(cmd.spawn()?))
    }
}

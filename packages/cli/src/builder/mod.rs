use crate::build::Build;
use crate::cli::serve::ServeArguments;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use dioxus_cli_config::{Platform, RuntimeCLIArguments};
use futures_util::stream::select_all;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::{path::PathBuf, process::Stdio};
use tokio::process::{Child, Command};

mod cargo;
mod fullstack;
mod prepare_html;
mod progress;
mod web;
pub use progress::{BuildMessage, MessageType, Stage, UpdateBuildProgress, UpdateStage};

/// A request for a project to be built
pub struct BuildRequest {
    /// Whether the build is for serving the application
    pub serve: bool,
    /// Whether this is a web build
    pub web: bool,
    /// The configuration for the crate we are building
    pub dioxus_crate: DioxusCrate,
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
    ) -> Vec<Self> {
        let build_arguments = build_arguments.into();
        let dioxus_crate = dioxus_crate.clone();
        let platform = build_arguments.platform();
        match platform {
            Platform::Web | Platform::Desktop => {
                let web = platform == Platform::Web;
                vec![Self {
                    serve,
                    web,
                    dioxus_crate,
                    build_arguments,
                    rust_flags: Default::default(),
                    target_dir: Default::default(),
                }]
            }
            Platform::StaticGeneration | Platform::Fullstack => {
                Self::new_fullstack(dioxus_crate, build_arguments, serve)
            }
            _ => unimplemented!("Unknown platform: {platform:?}"),
        }
    }

    pub async fn build_all_parallel(build_requests: Vec<BuildRequest>) -> Result<Vec<BuildResult>> {
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
}

#[derive(Debug, Clone)]
pub(crate) struct BuildResult {
    pub executable: PathBuf,
    pub web: bool,
    pub platform: Platform,
}

impl BuildResult {
    /// Open the executable if this is a native build
    pub fn open(
        &self,
        serve: &ServeArguments,
        fullstack_address: Option<SocketAddr>,
        workspace: &std::path::Path,
    ) -> std::io::Result<Option<Child>> {
        if self.web {
            return Ok(None);
        }

        let arguments = RuntimeCLIArguments::new(serve.address.address(), fullstack_address);
        let executable = self.executable.canonicalize()?;
        Ok(Some(
            Command::new(executable)
                // When building the fullstack server, we need to forward the serve arguments (like port) to the fullstack server through env vars
                .env(
                    dioxus_cli_config::__private::SERVE_ENV,
                    serde_json::to_string(&arguments).unwrap(),
                )
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .kill_on_drop(true)
                .current_dir(workspace)
                .spawn()?,
        ))
    }
}

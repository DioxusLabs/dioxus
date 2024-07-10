use crate::build::Build;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use cargo_metadata::diagnostic::Diagnostic;
use dioxus_cli_config::Platform;
use futures_util::{stream::FuturesUnordered, StreamExt};
use std::{path::PathBuf, time::Duration};
use tokio::process::{Child, Command};

mod cargo;
mod fullstack;
mod prepare_html;
mod progress;
mod web;
pub use progress::{BuildMessage, Stage, UpdateBuildProgress, UpdateStage};

/// A request for a project to be built
pub struct BuildRequest {
    /// Whether the build is for serving the application
    pub serve: bool,
    /// Whether this is a web build
    pub web: bool,
    /// The configuration for the crate we are building
    pub config: DioxusCrate,
    /// The arguments for the build
    pub build_arguments: Build,
    /// The rustc flags to pass to the build
    pub rust_flags: Option<String>,
    /// The target directory for the build
    pub target_dir: Option<PathBuf>,
}

impl BuildRequest {
    pub fn create(
        serve: bool,
        config: &DioxusCrate,
        build_arguments: impl Into<Build>,
    ) -> Vec<Self> {
        let build_arguments = build_arguments.into();
        let config = config.clone();
        let platform = build_arguments.platform.unwrap_or(Platform::Web);
        match platform {
            Platform::Web | Platform::Desktop => {
                let web = platform == Platform::Web;
                vec![Self {
                    serve,
                    web,
                    config,
                    build_arguments,
                    rust_flags: Default::default(),
                    target_dir: Default::default(),
                }]
            }
            Platform::StaticGeneration | Platform::Fullstack => {
                Self::new_fullstack(config, build_arguments, serve)
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
            build_progress.push((
                build_request.build_arguments.platform.unwrap_or_default(),
                rx,
            ));
            set.spawn(async move { build_request.build(tx).await });
        }

        // Watch the build progress as it comes in
        loop {
            let mut next = FuturesUnordered::new();
            for (platform, rx) in build_progress.iter_mut() {
                next.push(async { (*platform, rx.next().await) });
            }
            match next.next().await {
                Some((platform, update)) => {
                    if let Some(update) = update {
                        if multi_platform_build {
                            print!("{platform} build: ");
                            update.to_std_out();
                        } else {
                            update.to_std_out();
                        }
                    } else {
                        break;
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
                .map_err(|err| crate::Error::Unique("Failed to build project".to_owned()))??;
            all_results.push(result);
        }

        Ok(all_results)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BuildResult {
    pub executable: PathBuf,
    pub elapsed_time: Duration,
    pub web: bool,
}

impl BuildResult {
    /// Open the executable if this is a native build
    pub fn open(&self) -> std::io::Result<Option<Child>> {
        if self.web {
            return Ok(None);
        }
        Ok(Some(Command::new(&self.executable).spawn()?))
    }
}

use crate::build::{self, Build};
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use cargo_metadata::diagnostic::Diagnostic;
use dioxus_cli_config::Platform;
use futures_util::Future;
use manganis_cli_support::AssetManifest;
use std::cell::RefCell;
use std::sync::RwLock;
use std::{path::PathBuf, time::Duration};
use tokio::process::Child;

mod cargo;
mod fullstack;
mod prepare_html;
mod progress;
mod web;

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
}

impl BuildRequest {
    pub fn create(
        serve: bool,
        config: DioxusCrate,
        build_arguments: impl Into<Build>,
    ) -> Vec<BuildRequest> {
        let build_arguments = build_arguments.into();
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
                }]
            }
            Platform::StaticGeneration | Platform::Fullstack => {
                Self::new_fullstack(config, build_arguments, serve)
            }
            _ => unimplemented!("Unknown platform: {platform:?}"),
        }
    }
}

/// A handle to ongoing builds and then the spawned tasks themselves
#[derive(Default)]
pub struct Builder {
    /// The process that is building the application
    build_processes: RwLock<Vec<Child>>,
}

impl Builder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new build - killing the current one if it exists
    pub fn build(&self, build_request: BuildRequest) -> impl Future<Output = Result<BuildResult>> {
        async move {
            let result = tokio::task::spawn_blocking(move || build_request.build())
                .await
                .map_err(|err| {
                    crate::Error::Unique(
                        "Failed to build project with an unknown error {err:?}".to_string(),
                    )
                })??;

            Ok(result)
        }
    }

    /// Wait for any new updates to the builder - either it completed or gave us a mesage etc
    pub async fn wait(&mut self) {
        todo!()
    }

    /// Shutdown the current build process
    pub(crate) async fn shutdown(&self) -> Result<()> {
        let processes = std::mem::take(&mut *self.build_processes.write().unwrap());
        for mut build_process in processes {
            build_process.kill().await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub warnings: Vec<Diagnostic>,
    pub executable: Option<PathBuf>,
    pub elapsed_time: Duration,
    pub assets: Option<AssetManifest>,
}

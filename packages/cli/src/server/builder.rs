use crate::build::{self, Build};
use crate::builder::BuildRequest;
use crate::builder::BuildResult;
use crate::dioxus_crate::DioxusCrate;
use crate::serve::Serve;
use crate::Result;
use cargo_metadata::diagnostic::Diagnostic;
use dioxus_cli_config::Platform;
use futures_util::Future;
use manganis_cli_support::AssetManifest;
use std::cell::RefCell;
use std::{path::PathBuf, time::Duration};
use tokio::process::Child;
use tokio::sync::RwLock;
use tokio::task::JoinSet;

/// A handle to ongoing builds and then the spawned tasks themselves
pub struct Builder {
    /// The process that is building the application
    build_processes: Vec<std::thread::JoinHandle<()>>,
    /// The results of the build
    build_results: JoinSet<Result<BuildResult>>,
    /// The application we are building
    config: DioxusCrate,
    /// The arguments for the build
    build_arguments: Build,
}

impl Builder {
    /// Create a new builder
    pub fn new(config: &DioxusCrate, serve: &Serve) -> Self {
        let config = config.clone();
        let build_arguments = serve.build_arguments.clone();
        Self {
            build_processes: Default::default(),
            build_results: Default::default(),
            config,
            build_arguments,
        }
    }

    /// Start a new build - killing the current one if it exists
    pub async fn build(&mut self) -> Result<()> {
        self.shutdown().await?;
        let build_requests =
            BuildRequest::create(false, self.config.clone(), self.build_arguments.clone());

        for build_request in build_requests {
            let (tx, mut rx) = tokio::sync::oneshot::channel();
            let thread = std::thread::spawn(move || _ = tx.send(build_request.build()));
            self.build_processes.push(thread);
            self.build_results.spawn(async move {
                let result = rx.await.unwrap()?;
                Ok(result)
            });
        }

        Ok(())
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    pub async fn wait(&mut self) {
        _ = self.build_results.join_next().await;
    }

    /// Shutdown the current build process
    pub(crate) async fn shutdown(&mut self) -> Result<()> {
        for mut build_process in self.build_processes.drain(..) {
            todo!()
        }
        Ok(())
    }
}

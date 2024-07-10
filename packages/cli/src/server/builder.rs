use crate::build::Build;
use crate::builder::BuildRequest;
use crate::builder::BuildResult;
use crate::builder::UpdateBuildProgress;
use crate::dioxus_crate::DioxusCrate;
use crate::serve::Serve;
use crate::Result;
use dioxus_cli_config::Platform;
use futures_channel::mpsc::Receiver;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use tokio::task::JoinHandle;
use tokio::task::JoinSet;

/// A handle to ongoing builds and then the spawned tasks themselves
pub struct Builder {
    /// The results of the build
    build_results: Option<JoinHandle<Result<Vec<BuildResult>>>>,
    /// The progress of the builds
    build_progress: Vec<(Platform, UnboundedReceiver<UpdateBuildProgress>)>,
    /// The application we are building
    config: DioxusCrate,
    /// The arguments for the build
    build_arguments: Build,
}

impl Builder {
    /// Create a new builder
    pub fn new(config: &DioxusCrate, serve: &Serve) -> Self {
        let build_arguments = serve.build_arguments.clone();
        let config = config.clone();
        Self {
            build_results: None,
            build_progress: Vec::new(),
            config: config.clone(),
            build_arguments,
        }
    }

    /// Start a new build - killing the current one if it exists
    pub fn build(&mut self) {
        self.shutdown();
        let build_requests =
            BuildRequest::create(false, &self.config, self.build_arguments.clone());

        let mut set = tokio::task::JoinSet::new();
        for build_request in build_requests {
            let (tx, rx) = futures_channel::mpsc::unbounded();
            self.build_progress.push((
                build_request.build_arguments.platform.unwrap_or_default(),
                rx,
            ));
            set.spawn(async move { build_request.build(tx).await });
        }

        self.build_results = Some(tokio::spawn(async move {
            let mut all_results = Vec::new();
            while let Some(result) = set.join_next().await {
                all_results.push(result.map_err(|err| {
                    crate::Error::Unique(format!("Panic while building project: {err:?}"))
                })??);
            }
            Ok(all_results)
        }));
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    pub async fn wait(&mut self) -> Result<Option<(Platform, UpdateBuildProgress)>> {
        let Some(results) = self.build_results.as_mut() else {
            std::future::pending::<()>().await;
            return Ok(None);
        };

        // Wait for build progress
        let mut next = FuturesUnordered::new();
        for (platform, rx) in self.build_progress.iter_mut() {
            next.push(async { (*platform, rx.select_next_some().await) });
        }

        // Wait for the next build result
        tokio::select! {
            application = results => {
                // If we have a build result, open it
                let application = application.map_err(|_| crate::Error::Unique("Build failed".to_string()))?;
                for build_result in application? {
                    build_result.open()?;
                }
            }
            progress = next.next() => {
                // If we have a build progress, send it to the screen
                return Ok(progress)
            }
        }

        Ok(None)
    }

    /// Shutdown the current build process
    pub(crate) fn shutdown(&mut self) {
        if let Some(tasks) = self.build_results.take() {
            tasks.abort();
        }
        self.build_progress.clear();
    }
}

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
use futures_util::stream::select_all;
use futures_util::stream::{FusedStream, FuturesUnordered};
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
    serve: Serve,
}

impl Builder {
    /// Create a new builder
    pub fn new(config: &DioxusCrate, serve: &Serve) -> Self {
        let serve = serve.clone();
        let config = config.clone();
        Self {
            build_results: None,
            build_progress: Vec::new(),
            config: config.clone(),
            serve,
        }
    }

    /// Start a new build - killing the current one if it exists
    pub fn build(&mut self) {
        self.shutdown();
        let build_requests =
            BuildRequest::create(false, &self.config, self.serve.build_arguments.clone());

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
        let mut next = select_all(
            self.build_progress
                .iter_mut()
                .map(|(platform, rx)| rx.map(move |update| (*platform, update))),
        );

        // Wait for the next build result
        tokio::select! {
            application = results => {
                // If we have a build result, open it
                // let application = .map_err(|e| crate::Error::Unique("Build failed".to_string()))?;
                // let application = application.map_err(|e| crate::Error::Unique("Build failed".to_string()))?;

                match application {
                    Ok(Ok(application)) => {
                        for build_result in application {
                            _ = build_result.open(&self.serve.server_arguments);
                        }
                    }
                    Ok(Err(err)) => {
                        eprintln!("Build failed: {err:#?}");
                    }
                    Err(err) => {
                        eprintln!("Build join failed: {err:#?}");
                    }
                }
                // if let Ok(application) = application {
                //     for build_result in application {
                //         _ = build_result.open();
                //     }
                // }
                self.build_results = None;
                std::future::pending::<()>().await;
            }
            progress = next.next() => {
                // If we have a build progress, send it to the screen
                if let Some((platform, update)) = progress {
                    return Ok(Some((platform, update)));
                }
            }
        }

        // if results.is_finished() {
        //     _ = self.build_results.take();
        //     std::future::pending::<()>().await;
        //     return Ok(None);
        // }

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

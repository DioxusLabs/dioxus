use crate::builder::BuildRequest;
use crate::builder::BuildResult;
use crate::builder::TargetPlatform;
use crate::builder::UpdateBuildProgress;
use crate::dioxus_crate::DioxusCrate;
use crate::serve::next_or_pending;
use crate::serve::Serve;
use crate::Result;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::future::OptionFuture;
use futures_util::stream::select_all;
use futures_util::StreamExt;
use std::process::Stdio;
use tokio::{
    process::{Child, Command},
    task::JoinHandle,
};

/// A handle to ongoing builds and then the spawned tasks themselves
pub struct Builder {
    /// The results of the build
    build_results: Option<JoinHandle<Result<Vec<BuildResult>>>>,

    /// The progress of the builds
    build_progress: Vec<(TargetPlatform, UnboundedReceiver<UpdateBuildProgress>)>,

    /// The application we are building
    config: DioxusCrate,

    /// The arguments for the build
    serve: Serve,

    /// The children of the build process
    pub children: Vec<(TargetPlatform, Child)>,
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
            children: Vec::new(),
        }
    }

    /// Start a new build - killing the current one if it exists
    pub fn build(&mut self) -> Result<()> {
        self.shutdown();
        let build_requests =
            BuildRequest::create(true, &self.config, self.serve.build_arguments.clone())?;

        let mut set = tokio::task::JoinSet::new();

        for build_request in build_requests {
            let (mut tx, rx) = futures_channel::mpsc::unbounded();
            self.build_progress
                .push((build_request.target_platform, rx));
            set.spawn(async move {
                let res = build_request.build(tx.clone()).await;

                if let Err(err) = &res {
                    let _ = tx.start_send(UpdateBuildProgress {
                        stage: crate::builder::Stage::Finished,
                        update: crate::builder::UpdateStage::Failed(format!("{err}")),
                    });
                }

                res
            });
        }

        self.build_results = Some(tokio::spawn(async move {
            let mut all_results = Vec::new();
            while let Some(result) = set.join_next().await {
                let res = result.map_err(|err| {
                    crate::Error::Unique(format!("Panic while building project: {err:?}"))
                })??;

                all_results.push(res);
            }
            Ok(all_results)
        }));

        Ok(())
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    pub async fn wait(&mut self) -> Result<BuilderUpdate> {
        // Wait for build progress
        let mut next = select_all(
            self.build_progress
                .iter_mut()
                .map(|(platform, rx)| rx.map(move |update| (*platform, update))),
        );
        let next = next_or_pending(next.next());

        // The ongoing builds directly
        let results: OptionFuture<_> = self.build_results.as_mut().into();
        let results = next_or_pending(results);

        // The process exits
        let children_empty = self.children.is_empty();
        let process_exited = self
            .children
            .iter_mut()
            .map(|(target, child)| Box::pin(async move { (*target, child.wait().await) }));
        let process_exited = async move {
            if children_empty {
                return futures_util::future::pending().await;
            }
            futures_util::future::select_all(process_exited).await
        };

        // Wait for the next build result
        tokio::select! {
            build_results = results => {
                self.build_results = None;

                // If we have a build result, bubble it up to the main loop
                let build_results = build_results.map_err(|_| crate::Error::Unique("Build join failed".to_string()))??;

                Ok(BuilderUpdate::Ready { results: build_results })
            }
            (platform, update) = next => {
                // If we have a build progress, send it to the screen
                 Ok(BuilderUpdate::Progress { platform, update })
            }
            ((target, exit_status), _, _) = process_exited => {
                Ok(BuilderUpdate::ProcessExited { status: exit_status, target_platform: target })
            }
        }
    }

    /// Shutdown the current build process
    pub(crate) fn shutdown(&mut self) {
        for (_, mut child) in self.children.drain(..) {
            // Gracefully shtudown the desktop app
            // It might have a receiver to do some cleanup stuff
            if let Some(pid) = child.id() {
                // on unix, we can send a signal to the process to shut down
                #[cfg(unix)]
                {
                    _ = Command::new("kill")
                        .args(["-s", "TERM", &pid.to_string()])
                        .stderr(Stdio::null())
                        .stdout(Stdio::null())
                        .spawn();
                }

                // on windows, use the `taskkill` command
                #[cfg(windows)]
                {
                    _ = Command::new("taskkill")
                        .args(["/F", "/PID", &pid.to_string()])
                        .stderr(Stdio::null())
                        .stdout(Stdio::null())
                        .spawn();
                }
            }

            // Todo: add a timeout here to kill the process if it doesn't shut down within a reasonable time
            _ = child.start_kill();
        }

        if let Some(tasks) = self.build_results.take() {
            tasks.abort();
        }
        self.build_progress.clear();
    }
}

pub enum BuilderUpdate {
    Progress {
        platform: TargetPlatform,
        update: UpdateBuildProgress,
    },
    Ready {
        results: Vec<BuildResult>,
    },
    ProcessExited {
        target_platform: TargetPlatform,
        status: Result<std::process::ExitStatus, std::io::Error>,
    },
}

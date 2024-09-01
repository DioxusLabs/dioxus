use crate::builder::TargetPlatform;
use crate::builder::UpdateBuildProgress;
use crate::builder::{BuildReason, BuildRequest};
use crate::dioxus_crate::DioxusCrate;
use crate::serve::next_or_pending;
use crate::serve::Serve;
use crate::Result;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::future::OptionFuture;
use futures_util::StreamExt;
use std::{collections::HashMap, process::Stdio};
use tokio::{
    process::{Child, Command},
    task::JoinHandle,
};

use super::update::ServeUpdate;

/// A handle to ongoing builds and then the spawned tasks themselves
pub struct Builder {
    /// The results of the build
    ongoing: Option<JoinHandle<Result<Vec<BuildRequest>>>>,

    tx: UnboundedSender<UpdateBuildProgress>,
    rx: UnboundedReceiver<UpdateBuildProgress>,

    /// The application we are building
    config: DioxusCrate,

    /// The arguments for the build
    serve: Serve,

    /// The children of the build process
    pub finished: HashMap<TargetPlatform, BuildRequest>,
}

impl Builder {
    /// Create a new builder and immediately start a build
    pub fn start(config: &DioxusCrate, serve: &Serve) -> Result<Self> {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let mut builder = Self {
            tx,
            rx,
            ongoing: None,
            config: config.clone(),
            serve: serve.clone(),
            finished: Default::default(),
        };

        builder.build()?;

        Ok(builder)
    }

    /// Start a new build - killing the current one if it exists
    pub fn build(&mut self) -> Result<()> {
        self.shutdown();

        let build_requests = BuildRequest::create(
            BuildReason::Serve,
            &self.config,
            self.serve.build_arguments.clone(),
            self.tx.clone(),
        )?;

        let mut set = tokio::task::JoinSet::new();

        for build_request in build_requests {
            let mut tx = self.tx.clone();

            set.spawn(async move {
                let platform = build_request.target_platform.clone();
                let res = build_request.build().await;
                if let Err(err) = &res {
                    let _ = tx.start_send(UpdateBuildProgress {
                        stage: crate::builder::Stage::Finished,
                        update: crate::builder::UpdateStage::Failed(format!("{err}")),
                        platform,
                    });
                }

                res
            });
        }

        self.ongoing = Some(tokio::spawn(async move {
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
    ///
    /// Also listen for any input from the app's handle
    pub async fn wait(&mut self) -> ServeUpdate {
        // Wait for build progress
        let next = next_or_pending(self.rx.next());

        // The ongoing builds directly
        let results: OptionFuture<_> = self.ongoing.as_mut().into();
        let results = next_or_pending(results);

        todo!("wait for builds to be finished")

        // // The process exits
        // let children_empty = self.children.is_empty();
        // let process_exited = self
        //     .children
        //     .iter_mut()
        //     .map(|(target, child)| Box::pin(async move { (*target, child.wait().await) }));

        // let process_exited = async move {
        //     match children_empty {
        //         true => return futures_util::future::pending().await,
        //         false => futures_util::future::select_all(process_exited).await,
        //     }
        // };

        // // Wait for the next build result
        // tokio::select! {
        //     build_results = results => {
        //         self.ongoing = None;

        //         // If we have a build result, bubble it up to the main loop
        //         match build_results {
        //             Ok(Ok(build_results)) => ServeUpdate::BuildReady {  },
        //             Err(_ee) => ServeUpdate::BuildFailed { err: crate::Error::BuildFailed("Build join failed".to_string()) },
        //             Ok(Err(ee)) => ServeUpdate::BuildFailed { err: ee.into() },
        //         }
        //     }
        //     update = next => {
        //         // If we have a build progress, send it to the screen
        //          ServeUpdate::Progress { update }
        //     }
        //     ((target, exit_status), _, _) = process_exited => {
        //         ServeUpdate::ProcessExited { status: exit_status, target_platform: target }
        //     }
        // }
    }

    /// Shutdown the current build process
    pub(crate) fn shutdown(&mut self) {
        for (_target, app) in self.finished.drain() {
            let Some(mut child) = app.child else {
                continue;
            };

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

        if let Some(tasks) = self.ongoing.take() {
            tasks.abort();
        }
    }
}

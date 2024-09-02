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
    task::{JoinHandle, JoinSet},
};
use tokio_util::task::TaskTracker;

use super::update::ServeUpdate;

/// A handle to ongoing builds and then the spawned tasks themselves
pub struct Builder {
    /// Ongoing apps running in place
    ///
    /// They might be actively being being, running, or have exited.
    ///
    /// When a new full rebuild occurs, we will keep these requests here
    pub running: HashMap<TargetPlatform, BuildRequest>,

    tx: UnboundedSender<UpdateBuildProgress>,
    rx: UnboundedReceiver<UpdateBuildProgress>,

    /// The application we are building
    config: DioxusCrate,

    /// The arguments for the build
    serve: Serve,
}

impl Builder {
    /// Create a new builder and immediately start a build
    pub fn start(serve: &Serve, config: &DioxusCrate) -> Result<Self> {
        let (tx, rx) = futures_channel::mpsc::unbounded();

        let mut builder = Self {
            tx,
            rx,
            building: Default::default(),
            config: config.clone(),
            serve: serve.clone(),
            running: Default::default(),
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

        for build_request in build_requests {
            // Queue the build
            let platform = build_request.target_platform.clone();
            self.building.spawn(async move {
                // Run the build, but in a protected spawn, ensuring we can't produce panics and thus, joinerrors
                let res = tokio::spawn(build_request.build())
                    .await
                    .unwrap_or_else(|err| {
                        Err(crate::Error::Unique(format!(
                            "Panic while building project: {err:?}"
                        )))
                    });

                (platform, build_request)
            });
        }

        Ok(())
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    ///
    /// Also listen for any input from the app's handle
    pub async fn wait(&mut self) -> ServeUpdate {
        // Exits and stdout/stderr
        let processes = self.running.iter_mut().filter_map(|(target, request)| {
            let Some(child) = request.child else {
                return None;
            };

            Some(Box::pin(async move {
                //
                (*target, child.wait().await)
            }))
        });

        // Wait for the next build result
        tokio::select! {
            Some(update) = self.rx.next() => {
                ServeUpdate::Progress { update }
            }

            Some(Ok((target, build_result))) = self.building.join_next() => {
                match build_result {
                    Ok(build_result) => ServeUpdate::BuildReady { target, request: build_result },
                    Err(err) => ServeUpdate::BuildFailed { err, target },
                }
            }

            ((target, exit_status), _, _) = futures_util::future::select_all(processes) => {
                ServeUpdate::ProcessExited { status: exit_status, target_platform: target }
            }
        }
    }

    /// Shutdown the current build process
    pub(crate) fn shutdown(&mut self) {
        for (_target, app) in self.running.drain() {
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

        if let Some(tasks) = self.building.take() {
            tasks.abort();
        }
    }
}

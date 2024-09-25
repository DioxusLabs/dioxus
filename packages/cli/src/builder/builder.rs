use std::time::{Duration, Instant};

use crate::builder::*;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use crate::{build::BuildArgs, bundler::AppBundle};
use futures_util::StreamExt;
use progress::{ProgressRx, ProgressTx};
use tokio::task::JoinHandle;

/// The struct that handles the state of a build for an App.
/// Is currently designed to only handle one app and its server at a time, but feasibly could be
/// extended to support multiple apps and sidecars in the future.
pub(crate) struct Builder {
    /// The application we are building
    pub krate: DioxusCrate,

    pub request: BuildRequest,

    pub stage: BuildStage,

    pub build: JoinHandle<Result<AppBundle>>,

    /// Messages from the ongoing builds will be sent on this channel
    pub tx: ProgressTx,
    pub rx: ProgressRx,

    pub compiled_crates: usize,
    pub expected_crates: usize,
    pub bundling_progress: f64,
    pub compile_start: Option<Instant>,
    pub compile_end: Option<Instant>,
    pub bundle_start: Option<Instant>,
    pub bundle_end: Option<Instant>,
}

impl Builder {
    /// Create a new builder and immediately start a build
    pub(crate) fn start(krate: &DioxusCrate, args: BuildArgs) -> Result<Self> {
        let (tx, rx) = futures_channel::mpsc::unbounded();
        let request = BuildRequest::new(krate.clone(), args, tx.clone());
        Ok(Self {
            krate: krate.clone(),
            request: request.clone(),
            stage: BuildStage::Initializing,
            build: tokio::spawn(async move {
                let res = request.build().await;

                // The first launch gets some extra logging :)
                if res.is_ok() {
                    tracing::info!("Build completed successfully, launching app! 💫")
                }

                res
            }),
            tx,
            rx,
            compiled_crates: 0,
            expected_crates: 1,
            bundling_progress: 0.0,
            compile_start: Some(Instant::now()),
            compile_end: None,
            bundle_start: None,
            bundle_end: None,
        })
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    pub(crate) async fn wait(&mut self) -> BuildUpdate {
        // Wait for the build to finish or for it to eminate a status message
        let update = tokio::select! {
            bundle = (&mut self.build) => {
                self.build = tokio::task::spawn(std::future::pending());
                match bundle {
                    Ok(Ok(bundle)) => BuildUpdate::BuildReady { bundle },
                    Ok(Err(err)) => BuildUpdate::BuildFailed { err },
                    Err(_) => BuildUpdate::BuildFailed { err: crate::Error::from(anyhow::anyhow!("Build panicked!")) },
                }
            },
            Some(progress) = self.rx.next() => progress,
        };

        // Update the internal stage of the build so the UI can render it
        match &update {
            BuildUpdate::Progress { stage } => {
                // Prevent updates from flowing in after the build has already finished
                if !self.is_finished() {
                    self.stage = stage.clone();
                }

                match stage {
                    BuildStage::Initializing => {
                        self.compiled_crates = 0;
                        self.bundling_progress = 0.0;
                    }
                    BuildStage::Starting {
                        server,
                        crate_count,
                    } => {
                        self.expected_crates += crate_count;
                    }
                    BuildStage::InstallingTooling {} => {}
                    BuildStage::Compiling { current, total, .. } => {
                        self.compiled_crates += 1;

                        if self.compile_start.is_none() {
                            self.compile_start = Some(Instant::now());
                        }
                    }
                    BuildStage::Bundling {} => {
                        self.bundling_progress = 0.0;
                        self.compile_end = Some(Instant::now());
                        self.bundle_start = Some(Instant::now());
                    }
                    BuildStage::OptimizingWasm {} => {}
                    BuildStage::OptimizingAssets {} => {}
                    BuildStage::CopyingAssets { current, total, .. } => {
                        self.bundling_progress = *current as f64 / *total as f64;
                    }
                    BuildStage::Success => {
                        self.compiled_crates = self.expected_crates;
                        self.bundling_progress = 1.0;
                    }
                    BuildStage::Failed => {
                        self.compiled_crates = self.expected_crates;
                        self.bundling_progress = 1.0;
                    }
                    BuildStage::Aborted => {}
                    BuildStage::Restarting => {
                        self.compiled_crates = 0;
                        self.expected_crates = 1;
                        self.bundling_progress = 0.0;
                    }
                    BuildStage::RunningBindgen {} => {
                        self.bundling_progress = 0.5;
                    }
                }
            }
            BuildUpdate::CompilerMessage { .. } => {}
            BuildUpdate::BuildReady { .. } => {
                self.compiled_crates = self.expected_crates;
                self.bundling_progress = 1.0;
                self.stage = BuildStage::Success;
                self.bundle_end = Some(Instant::now());
            }
            BuildUpdate::BuildFailed { .. } => {
                tracing::debug!("Setting builder to failed state");
                self.stage = BuildStage::Failed;
            }
        }

        update
    }

    /// Wait for the build to finish, returning the final bundle
    pub(crate) async fn finish(&mut self) -> Result<AppBundle> {
        loop {
            match self.wait().await {
                BuildUpdate::BuildReady { bundle } => return Ok(bundle),
                BuildUpdate::BuildFailed { err } => return Err(err),
                BuildUpdate::Progress { .. } => {
                    // maybe log this?
                }
                BuildUpdate::CompilerMessage { .. } => {
                    // maybe log this?
                }
            }
        }
    }

    /// Restart this builder with new build arguments.
    pub(crate) fn rebuild(&mut self, args: BuildArgs) {
        // Abort all the ongoing builds, cleaning up any loose artifacts and waiting to cleanly exit
        self.abort_all();

        // And then start a new build, resetting our progress/stage to the beginning and replacing the old tokio task
        let request = BuildRequest::new(self.krate.clone(), args, self.tx.clone());
        self.request = request.clone();
        self.stage = BuildStage::Restarting;

        // This build doesn't have any extra special logging - rebuilds would get pretty noisy
        self.build = tokio::spawn(async move { request.build().await });
    }

    /// Shutdown the current build process
    ///
    /// todo: might want to use a cancellation token here to allow cleaner shutdowns
    pub(crate) fn abort_all(&mut self) {
        self.build.abort();
        self.stage = BuildStage::Aborted;
        self.compiled_crates = 0;
        self.expected_crates = 1;
        self.bundling_progress = 0.0;
        self.compile_start = None;
        self.bundle_start = None;
        self.bundle_end = None;
        self.compile_end = None;
    }

    /// Get the duration of the compile phase
    pub fn compile_duration(&self) -> Option<Duration> {
        Some(
            self.compile_end
                .unwrap_or_else(Instant::now)
                .duration_since(self.compile_start?),
        )
    }

    pub fn bundle_duration(&self) -> Option<Duration> {
        Some(
            self.bundle_end
                .unwrap_or_else(Instant::now)
                .duration_since(self.bundle_start?),
        )
    }

    pub fn compile_progress(&self) -> f64 {
        self.compiled_crates as f64 / self.expected_crates as f64
    }

    pub(crate) fn total_build_time(&self) -> Option<Duration> {
        Some(self.compile_duration()? + self.bundle_duration()?)
    }

    fn is_finished(&self) -> bool {
        match self.stage {
            BuildStage::Success => true,
            BuildStage::Failed => true,
            BuildStage::Aborted => true,
            BuildStage::Restarting => true,
            _ => false,
        }
    }
}

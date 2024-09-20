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

    pub compile_progress: f64,
    pub optimize_progress: f64,
    pub bundling_progress: f64,
}

impl Builder {
    /// Create a new builder and immediately start a build
    pub(crate) fn start(krate: &DioxusCrate, args: BuildArgs) -> Result<Self> {
        let (tx, rx) = futures_channel::mpsc::unbounded();
        let request = BuildRequest::new(&krate, args.clone(), tx.clone());
        Ok(Self {
            krate: krate.clone(),
            request: request.clone(),
            stage: BuildStage::Initializing,
            build: tokio::spawn(request.build()),
            tx,
            rx,
            compile_progress: 0.0,
            optimize_progress: 0.0,
            bundling_progress: 0.0,
        })
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    ///
    /// Also listen for any input from the app's handle
    ///
    /// Returns immediately with `Finished` if there are no more builds to run - don't poll-loop this!
    pub(crate) async fn wait(&mut self) -> BuildUpdate {
        if self.build.is_finished() {
            std::future::pending().await
        }

        // Wait for the build to finish or for it to eminate a status message
        let update = tokio::select! {
            bundle = (&mut self.build) => match bundle {
                Ok(Ok(bundle)) => BuildUpdate::BuildReady { bundle },
                Ok(Err(err)) => BuildUpdate::BuildFailed { err },
                Err(_) => BuildUpdate::BuildFailed { err: crate::Error::from(anyhow::anyhow!("Build panicked!")) },
            },
            Some(progress) = self.rx.next() => progress,
        };

        // Update the internal stage of the build so the UI can render it
        match &update {
            BuildUpdate::Progress { stage } => {
                self.stage = stage.clone();
                match stage {
                    BuildStage::Initializing => {
                        self.compile_progress = 0.0;
                        self.optimize_progress = 0.0;
                        self.bundling_progress = 0.0;
                    }
                    BuildStage::InstallingTooling {} => {
                        self.compile_progress = 0.1;
                    }
                    BuildStage::Compiling { current, total } => {
                        self.compile_progress = *current as f64 / *total as f64;
                    }
                    BuildStage::OptimizingWasm {} => {
                        self.optimize_progress = 0.3;
                    }
                    BuildStage::OptimizingAssets {} => {
                        self.optimize_progress = 0.7;
                    }
                    BuildStage::CopyingAssets { current, total } => {
                        self.bundling_progress = *current as f64 / *total as f64;
                    }
                    BuildStage::Success => {
                        self.compile_progress = 1.0;
                        self.optimize_progress = 1.0;
                        self.bundling_progress = 1.0;
                    }
                    BuildStage::Failed => {}
                    BuildStage::Aborted => {}
                    BuildStage::Restarting => {
                        self.compile_progress = 0.0;
                        self.optimize_progress = 0.0;
                        self.bundling_progress = 0.0;
                    }
                }
            }
            BuildUpdate::Message {} => {}
            BuildUpdate::BuildReady { .. } => {
                self.compile_progress = 1.0;
                self.optimize_progress = 1.0;
                self.bundling_progress = 1.0;
            }
            BuildUpdate::BuildFailed { .. } => {}
        }

        update
    }

    /// Wait for the build to finish, returning the final bundle
    pub(crate) async fn finish(&mut self) -> Result<AppBundle> {
        loop {
            let next = self.wait().await;
            match next {
                BuildUpdate::BuildReady { bundle } => return Ok(bundle),
                BuildUpdate::BuildFailed { err } => return Err(err),
                BuildUpdate::Progress { .. } => {
                    // maybe log this?
                }
                BuildUpdate::Message {} => {
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
        let request = BuildRequest::new(&self.krate, args.clone(), self.tx.clone());
        self.request = request.clone();
        self.stage = BuildStage::Restarting;
        self.build = tokio::spawn(request.build());
    }

    /// Shutdown the current build process
    ///
    /// todo: might want to use a cancellation token here to allow cleaner shutdowns
    pub(crate) fn abort_all(&mut self) {
        self.build.abort();
        self.stage = BuildStage::Aborted;
        self.compile_progress = 0.0;
        self.optimize_progress = 0.0;
        self.bundling_progress = 0.0;
    }
}

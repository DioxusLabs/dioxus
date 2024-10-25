use crate::{
    AppBundle, BuildArgs, BuildRequest, BuildStage, BuildUpdate, DioxusCrate, Platform, ProgressRx,
    ProgressTx, Result,
};
use std::time::{Duration, Instant};

/// The component of the serve engine that watches ongoing builds and manages their state, handle,
/// and progress.
///
/// Previously, the builder allowed multiple apps to be built simultaneously, but this newer design
/// simplifies the code and allows only one app and its server to be built at a time.
///
/// Here, we track the number of crates being compiled, assets copied, the times of these events, and
/// other metadata that gives us useful indicators for the UI.
pub(crate) struct Builder {
    // Components of the build
    pub krate: DioxusCrate,
    pub request: BuildRequest,
    pub build: tokio::task::JoinHandle<Result<AppBundle>>,
    pub tx: ProgressTx,
    pub rx: ProgressRx,

    // Metadata about the build that needs to be managed by watching build updates
    // used to render the TUI
    pub stage: BuildStage,
    pub compiled_crates: usize,
    pub compiled_crates_server: usize,
    pub expected_crates: usize,
    pub expected_crates_server: usize,
    pub bundling_progress: f64,
    pub compile_start: Option<Instant>,
    pub compile_end: Option<Instant>,
    pub compile_end_server: Option<Instant>,
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
                // On the first build, we want to verify the tooling
                // We wont bother verifying on subsequent builds
                request.verify_tooling().await?;

                let res = request.build_all().await;

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
            expected_crates_server: 1,
            compiled_crates_server: 0,
            bundling_progress: 0.0,
            compile_start: Some(Instant::now()),
            compile_end: None,
            compile_end_server: None,
            bundle_start: None,
            bundle_end: None,
        })
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    pub(crate) async fn wait(&mut self) -> BuildUpdate {
        use futures_util::StreamExt;

        // Wait for the build to finish or for it to emit a status message
        let update = tokio::select! {
            Some(progress) = self.rx.next() => progress,
            bundle = (&mut self.build) => {
                // Replace the build with an infinitely pending task so we can select it again without worrying about deadlocks/spins
                self.build = tokio::task::spawn(std::future::pending());
                match bundle {
                    Ok(Ok(bundle)) => BuildUpdate::BuildReady { bundle },
                    Ok(Err(err)) => BuildUpdate::BuildFailed { err },
                    Err(err) => BuildUpdate::BuildFailed { err: crate::Error::Runtime(format!("Build panicked! {:?}", err)) },
                }
            },
        };

        tracing::trace!("Build update: {update:?}");

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
                        self.compiled_crates_server = 0;
                        self.bundling_progress = 0.0;
                    }
                    BuildStage::Starting {
                        crate_count,
                        platform,
                    } => {
                        if *platform == Platform::Server {
                            self.expected_crates_server = *crate_count;
                        } else {
                            self.expected_crates = *crate_count;
                        }
                    }
                    BuildStage::InstallingTooling {} => {}
                    BuildStage::Compiling {
                        current,
                        total,
                        platform,
                        ..
                    } => {
                        if *platform == Platform::Server {
                            self.compiled_crates_server = *current;
                            self.expected_crates_server = *total;
                        } else {
                            self.compiled_crates = *current;
                            self.expected_crates = *total;
                        }

                        if self.compile_start.is_none() {
                            self.compile_start = Some(Instant::now());
                        }
                    }
                    BuildStage::Bundling {} => {
                        self.complete_compile();
                        self.bundling_progress = 0.0;
                        self.bundle_start = Some(Instant::now());
                    }
                    BuildStage::OptimizingWasm {} => {}
                    BuildStage::CopyingAssets { current, total, .. } => {
                        self.bundling_progress = *current as f64 / *total as f64;
                    }
                    BuildStage::Success => {
                        self.compiled_crates = self.expected_crates;
                        self.compiled_crates_server = self.expected_crates_server;
                        self.bundling_progress = 1.0;
                    }
                    BuildStage::Failed => {
                        self.compiled_crates = self.expected_crates;
                        self.compiled_crates_server = self.expected_crates_server;
                        self.bundling_progress = 1.0;
                    }
                    BuildStage::Aborted => {}
                    BuildStage::Restarting => {
                        self.compiled_crates = 0;
                        self.compiled_crates_server = 0;
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
                self.compiled_crates_server = self.expected_crates_server;
                self.bundling_progress = 1.0;
                self.stage = BuildStage::Success;

                self.complete_compile();
                self.bundle_end = Some(Instant::now());
            }
            BuildUpdate::BuildFailed { .. } => {
                tracing::debug!("Setting builder to failed state");
                self.stage = BuildStage::Failed;
            }
        }

        update
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
        self.build = tokio::spawn(async move { request.build_all().await });
    }

    /// Shutdown the current build process
    ///
    /// todo: might want to use a cancellation token here to allow cleaner shutdowns
    pub(crate) fn abort_all(&mut self) {
        self.build.abort();
        self.stage = BuildStage::Aborted;
        self.compiled_crates = 0;
        self.compiled_crates_server = 0;
        self.expected_crates = 1;
        self.bundling_progress = 0.0;
        self.compile_start = None;
        self.bundle_start = None;
        self.bundle_end = None;
        self.compile_end = None;
    }

    /// Wait for the build to finish, returning the final bundle
    /// Should only be used by code that's not interested in the intermediate updates and only cares about the final bundle
    ///
    /// todo(jon): maybe we want to do some logging here? The build/bundle/run screens could be made to
    /// use the TUI output for prettier outputs.
    pub(crate) async fn finish(&mut self) -> Result<AppBundle> {
        loop {
            match self.wait().await {
                BuildUpdate::BuildReady { bundle } => return Ok(bundle),
                BuildUpdate::BuildFailed { err } => return Err(err),
                BuildUpdate::Progress { .. } => {}
                BuildUpdate::CompilerMessage { .. } => {}
            }
        }
    }

    fn complete_compile(&mut self) {
        if self.compile_end.is_none() {
            self.compiled_crates = self.expected_crates;
            self.compile_end = Some(Instant::now());
            self.compile_end_server = Some(Instant::now());
        }
    }

    /// Get the total duration of the build, if all stages have completed
    pub(crate) fn total_build_time(&self) -> Option<Duration> {
        Some(self.compile_duration()? + self.bundle_duration()?)
    }

    pub(crate) fn compile_duration(&self) -> Option<Duration> {
        Some(
            self.compile_end
                .unwrap_or_else(Instant::now)
                .duration_since(self.compile_start?),
        )
    }

    pub(crate) fn bundle_duration(&self) -> Option<Duration> {
        Some(
            self.bundle_end
                .unwrap_or_else(Instant::now)
                .duration_since(self.bundle_start?),
        )
    }

    /// Return a number between 0 and 1 representing the progress of the app build
    pub(crate) fn compile_progress(&self) -> f64 {
        self.compiled_crates as f64 / self.expected_crates as f64
    }

    /// Return a number between 0 and 1 representing the progress of the server build
    pub(crate) fn server_compile_progress(&self) -> f64 {
        self.compiled_crates_server as f64 / self.expected_crates_server as f64
    }

    pub(crate) fn bundle_progress(&self) -> f64 {
        self.bundling_progress
    }

    fn is_finished(&self) -> bool {
        match self.stage {
            BuildStage::Success => true,
            BuildStage::Failed => true,
            BuildStage::Aborted => true,
            BuildStage::Restarting => false,
            _ => false,
        }
    }
}

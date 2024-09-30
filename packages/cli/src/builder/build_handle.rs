use std::time::{Duration, Instant};

use crate::builder::*;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use crate::{build::TargetArgs, bundler::AppBundle};
use futures_util::StreamExt;
use progress::{ProgressRx, ProgressTx};
use result::BuildResult;
use tokio::task::JoinHandle;

pub struct BuildHandle {
    pub request: BuildRequest,

    pub stage: BuildStage,

    pub build: JoinHandle<Result<BuildResult>>,

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

impl BuildHandle {
    fn new() -> Self {
        // let (tx, rx) = futures_channel::mpsc::unbounded();

        todo!()

        // Self {
        //     compiled_crates: 0,
        //     expected_crates: 1,
        //     expected_crates_server: 1,
        //     compiled_crates_server: 0,

        //     bundling_progress: 0.0,
        //     compile_start: Some(Instant::now()),
        //     compile_end: None,
        //     compile_end_server: None,
        //     bundle_start: None,
        //     bundle_end: None,

        // }
    }

    pub async fn wait(&mut self) -> BuildUpdate {
        let update = tokio::select! {
            Some(progress) = self.rx.next() => progress,
            else => futures_util::future::pending().await,
        };

        // Update the internal stage of the build so the UI can render it
        match &update {
            BuildUpdate::Progress { stage, .. } => {
                // Prevent updates from flowing in after the build has already finished
                if !self.is_finished() {
                    self.stage = stage.clone();
                }

                match stage {
                    BuildStage::NotStarted => {}
                    BuildStage::Initializing => {
                        self.compiled_crates = 0;
                        self.bundling_progress = 0.0;
                    }
                    BuildStage::Starting { crate_count } => {
                        // self.expected_crates += crate_count;
                    }
                    BuildStage::InstallingTooling {} => {}
                    BuildStage::Compiling { current, total, .. } => {
                        // if *server {
                        //     self.compiled_crates_server = *current;
                        //     self.expected_crates_server = *total;
                        // } else {
                        //     self.compiled_crates = *current;
                        //     self.expected_crates = *total;
                        // }

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
                    _ => {}
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

    pub fn compile_progress(&self) -> f64 {
        self.compiled_crates as f64 / self.expected_crates as f64
    }

    /// Get the duration of the compile phase
    pub fn build_duration(&self) -> Option<Duration> {
        Some(
            self.compile_end
                .unwrap_or_else(Instant::now)
                .duration_since(self.compile_start?),
        )
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

//! Report progress about the build to the user. We use channels to report progress back to the CLI.
use crate::{platform, AppBundle, BuildRequest, Platform};
use anyhow::Context;
use cargo_metadata::{diagnostic::Diagnostic, CompilerMessage, Message};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use serde::Deserialize;
use std::ops::Deref;
use std::path::PathBuf;
use std::process::Stdio;
use std::{fmt::Display, path::Path};
use tokio::{io::AsyncBufReadExt, process::Command};
use tracing::Level;

pub(crate) type ProgressTx = UnboundedSender<BuildUpdate>;
pub(crate) type ProgressRx = UnboundedReceiver<BuildUpdate>;

#[derive(Debug)]
pub(crate) enum BuildUpdate {
    Progress { stage: BuildStage },
    CompilerMessage { message: CompilerMessage },
    BuildReady { bundle: AppBundle },
    BuildFailed { err: crate::Error },
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BuildStage {
    Initializing,
    Starting {
        platform: Platform,
        crate_count: usize,
    },
    InstallingTooling {},
    Compiling {
        platform: Platform,
        current: usize,
        total: usize,
        krate: String,
    },
    Bundling {},
    RunningBindgen {},
    OptimizingWasm {},
    OptimizingAssets {},
    CopyingAssets {
        current: usize,
        total: usize,
        path: PathBuf,
    },
    Success,
    Failed,
    Aborted,
    Restarting,
}

impl BuildRequest {
    pub(crate) fn status_wasm_bindgen_start(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::RunningBindgen {},
        });
    }
    pub(crate) fn status_wasm_opt_start(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::RunningBindgen {},
        });
    }

    pub(crate) fn status_start_bundle(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::Bundling {},
        });
    }

    pub(crate) fn status_build_diagnostic(&self, message: CompilerMessage) {
        _ = self
            .progress
            .unbounded_send(BuildUpdate::CompilerMessage { message });
    }

    pub(crate) fn status_build_message(&self, line: String) {
        // _ = self.progress.unbounded_send(BuildUpdate::Progress {
        //     platform: self.platform(),
        //     stage: BuildStage::Compiling,
        //     update: UpdateStage::AddMessage(BuildMessage {
        //         level: Level::DEBUG,
        //         message: MessageType::Text(line),
        //         source: MessageSource::Build,
        //     }),
        // });
    }

    pub(crate) fn status_build_progress(
        &self,
        count: usize,
        total: usize,
        name: String,
        platform: Platform,
    ) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::Compiling {
                current: count,
                total,
                krate: name,
                platform,
            },
        });
    }

    pub(crate) fn status_starting_build(&self, crate_count: usize) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::Starting {
                platform: self.build.platform(),
                crate_count,
            },
        });
    }

    pub(crate) fn status_build_finished(&self) {
        // tracing::info!("🚩 Build completed: [{}]", self.krate.out_dir().display());

        todo!()
        // _ = self.progress.unbounded_send(BuildUpdate::Progress {
        //     stage: BuildStage::Finished,
        //     progress: 1.0,
        // });
    }

    pub(crate) fn status_copying_asset(&self, cur: usize, total: usize, asset: &Path) {
        // Update the progress
        // _ = self.progress.unbounded_send(UpdateBuildProgress {
        //     stage: Stage::OptimizingAssets,
        //     update: UpdateStage::AddMessage(BuildMessage {
        //         level: Level::INFO,
        //         message: MessageType::Text(format!(
        //             "Optimized static asset {}",
        //             asset.display()
        //         )),
        //         source: MessageSource::Build,
        //     }),
        //     platform: self.target_platform,
        // });
    }

    pub(crate) fn status_finished_asset(&self, idx: usize, total: usize, asset: &Path) {
        // Update the progress
        // _ = self.progress.unbounded_send(UpdateBuildProgress {
        //     stage: Stage::OptimizingAssets,
        //     update: UpdateStage::SetProgress(finished as f64 / asset_count as f64),
        //     platform: self.target_platform,
        // });
    }

    pub(crate) fn status_optimizing_wasm(&self) {
        todo!()
    }
}

//! Report progress about the build to the user. We use channels to report progress back to the CLI.
use crate::{bundler::AppBundle, BuildRequest, Platform};
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
        server: bool,
        crate_count: usize,
    },
    InstallingTooling {},
    Compiling {
        current: usize,
        total: usize,
        krate: String,
        server: bool,
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
    pub(crate) fn status_wasm_bindgen(&self) {
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
        server: bool,
    ) {
        self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::Compiling {
                current: count,
                total,
                krate: name,
                server,
            },
        });
    }

    pub(crate) fn status_starting_build(&self, server: bool, crate_count: usize) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::Starting {
                server,
                crate_count,
            },
        });
    }

    /// Try to get the unit graph for the crate. This is a nightly only feature which may not be available with the current version of rustc the user has installed.
    pub(crate) async fn get_unit_count(&self, is_server: bool) -> crate::Result<usize> {
        #[derive(Debug, Deserialize)]
        struct UnitGraph {
            units: Vec<serde_json::Value>,
        }

        let output = tokio::process::Command::new("cargo")
            .arg("+nightly")
            .arg("build")
            .arg("--unit-graph")
            .arg("-Z")
            .arg("unstable-options")
            .args(self.build_arguments(is_server))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get unit count").into());
        }

        let output_text = String::from_utf8(output.stdout).context("Failed to get unit count")?;
        let graph: UnitGraph =
            serde_json::from_str(&output_text).context("Failed to get unit count")?;

        Ok(graph.units.len())
    }

    /// Get an estimate of the number of units in the crate. If nightly rustc is not available, this will return an estimate of the number of units in the crate based on cargo metadata.
    /// TODO: always use https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#unit-graph once it is stable
    pub(crate) async fn get_unit_count_estimate(&self, is_server: bool) -> usize {
        // Try to get it from nightly
        self.get_unit_count(is_server).await.unwrap_or_else(|_| {
            // Otherwise, use cargo metadata
            (self
                .krate
                .krates
                .krates_filtered(krates::DepKind::Dev)
                .iter()
                .map(|k| k.targets.len())
                .sum::<usize>() as f64
                / 3.5) as usize
        })
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

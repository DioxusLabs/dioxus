//! Report progress about the build to the user. We use channels to report progress back to the CLI.
use crate::{AppBundle, BuildRequest, Platform, TraceSrc};
use cargo_metadata::CompilerMessage;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use std::path::PathBuf;

pub(crate) type ProgressTx = UnboundedSender<BuildUpdate>;
pub(crate) type ProgressRx = UnboundedReceiver<BuildUpdate>;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
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
        tracing::trace!(dx_src = ?TraceSrc::Cargo, "{line}");
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

    pub(crate) fn status_copying_asset(&self, current: usize, total: usize, path: PathBuf) {
        tracing::trace!("Status copying asset {current}/{total} from {path:?}");
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::CopyingAssets {
                current,
                total,
                path,
            },
        });
    }

    pub(crate) fn status_optimizing_wasm(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::OptimizingWasm {},
        });
    }

    pub(crate) fn status_installing_tooling(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::InstallingTooling {},
        });
    }
}

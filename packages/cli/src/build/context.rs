//! Report progress about the build to the user. We use channels to report progress back to the CLI.

use super::BuildMode;
use crate::{BuildArtifacts, BuildStage, Error, TraceSrc};
use cargo_metadata::CompilerMessage;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, process::ExitStatus};

/// The context of the build process. While the BuildRequest is a "plan" for the build, the BuildContext
/// provides some dynamic configuration that is only known at runtime. For example, the Progress channel
/// and the BuildMode can change while serving.
///
/// The structure of this is roughly taken from cargo itself which uses a similar pattern.
#[derive(Debug, Clone)]
pub struct BuildContext {
    pub tx: ProgressTx,
    pub mode: BuildMode,
}

pub type ProgressTx = UnboundedSender<BuilderUpdate>;
pub type ProgressRx = UnboundedReceiver<BuilderUpdate>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct BuildId(pub(crate) usize);
impl BuildId {
    pub const CLIENT: Self = Self(0);
    pub const SERVER: Self = Self(1);
}

#[allow(clippy::large_enum_variant)]
pub enum BuilderUpdate {
    Progress {
        stage: BuildStage,
    },

    CompilerMessage {
        message: CompilerMessage,
    },

    BuildReady {
        bundle: BuildArtifacts,
    },

    BuildFailed {
        err: Error,
    },

    /// A running process has received a stdout.
    /// May or may not be a complete line - do not treat it as a line. It will include a line if it is a complete line.
    ///
    /// We will poll lines and any content in a 50ms interval
    StdoutReceived {
        msg: String,
    },

    /// A running process has received a stderr.
    /// May or may not be a complete line - do not treat it as a line. It will include a line if it is a complete line.
    ///
    /// We will poll lines and any content in a 50ms interval
    StderrReceived {
        msg: String,
    },

    ProcessExited {
        status: ExitStatus,
    },
}

impl BuildContext {
    pub(crate) fn status_wasm_bindgen_start(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::RunningBindgen,
        });
    }

    pub(crate) fn status_splitting_bundle(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::SplittingBundle,
        });
    }

    pub(crate) fn status_start_bundle(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::Bundling,
        });
    }

    pub(crate) fn status_running_gradle(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::RunningGradle,
        })
    }

    pub(crate) fn status_build_diagnostic(&self, message: CompilerMessage) {
        _ = self
            .tx
            .unbounded_send(BuilderUpdate::CompilerMessage { message });
    }

    pub(crate) fn status_build_error(&self, line: String) {
        tracing::error!(dx_src = ?TraceSrc::Cargo, "{line}");
    }

    pub(crate) fn status_build_message(&self, line: String) {
        tracing::trace!(dx_src = ?TraceSrc::Cargo, "{line}");
    }

    pub(crate) fn status_build_progress(&self, count: usize, total: usize, name: String) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::Compiling {
                current: count,
                total,
                krate: name,
            },
        });
    }

    pub(crate) fn status_starting_build(&self, crate_count: usize) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::Starting {
                patch: matches!(self.mode, BuildMode::Thin { .. }),
                crate_count,
            },
        });
    }

    pub(crate) fn status_starting_link(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::Linking,
        });
    }

    pub(crate) fn status_copied_asset(
        progress: &UnboundedSender<BuilderUpdate>,
        current: usize,
        total: usize,
        path: PathBuf,
    ) {
        _ = progress.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::CopyingAssets {
                current,
                total,
                path,
            },
        });
    }

    pub(crate) fn status_optimizing_wasm(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::OptimizingWasm,
        });
    }

    pub(crate) fn status_hotpatching(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::Hotpatching,
        });
    }

    pub(crate) fn status_installing_tooling(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::InstallingTooling,
        });
    }

    pub(crate) fn status_compressing_assets(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::CompressingAssets,
        });
    }
    pub(crate) fn status_extracting_assets(&self) {
        _ = self.tx.unbounded_send(BuilderUpdate::Progress {
            stage: BuildStage::ExtractingAssets,
        });
    }
}

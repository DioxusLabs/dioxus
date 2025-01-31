//! Report progress about the build to the user. We use channels to report progress back to the CLI.
use crate::{AppBundle, BuildRequest, BuildStage, Platform, TraceSrc};
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

impl BuildRequest {
    pub(crate) fn status_wasm_bindgen_start(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::RunningBindgen {},
        });
    }

    pub(crate) fn status_start_bundle(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::Bundling {},
        });
    }

    pub(crate) fn status_running_gradle(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::RunningGradle,
        })
    }

    pub(crate) fn status_build_diagnostic(&self, message: CompilerMessage) {
        _ = self
            .progress
            .unbounded_send(BuildUpdate::CompilerMessage { message });
    }

    pub(crate) fn status_build_error(&self, line: String) {
        tracing::error!(dx_src = ?TraceSrc::Cargo, "{line}");
    }

    pub(crate) fn status_build_message(&self, line: String) {
        tracing::trace!(dx_src = ?TraceSrc::Cargo, "{line}");
    }

    pub(crate) fn status_build_progress(&self, count: usize, total: usize, name: String) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::Compiling {
                current: count,
                total,
                krate: name,
                is_server: self.is_server(),
            },
        });
    }

    pub(crate) fn status_starting_build(&self, crate_count: usize) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::Starting {
                is_server: self.build.platform() == Platform::Server,
                crate_count,
            },
        });
    }

    pub(crate) fn status_copied_asset(
        progress: &UnboundedSender<BuildUpdate>,
        current: usize,
        total: usize,
        path: PathBuf,
    ) {
        _ = progress.unbounded_send(BuildUpdate::Progress {
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

    pub(crate) fn status_prerendering_routes(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::PrerenderingRoutes {},
        });
    }

    pub(crate) fn status_installing_tooling(&self) {
        _ = self.progress.unbounded_send(BuildUpdate::Progress {
            stage: BuildStage::InstallingTooling {},
        });
    }

    pub(crate) fn is_server(&self) -> bool {
        self.build.platform() == Platform::Server
    }
}

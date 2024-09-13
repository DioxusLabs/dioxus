//! Report progress about the build to the user. We use channels to report progress back to the CLI.
use super::{BuildRequest, Platform};
use anyhow::Context;
use cargo_metadata::{diagnostic::Diagnostic, Message};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use serde::Deserialize;
use std::ops::Deref;
use std::path::PathBuf;
use std::process::Stdio;
use std::{fmt::Display, path::Path};
use tokio::{io::AsyncBufReadExt, process::Command};
use tracing::Level;

impl BuildRequest {
    pub(crate) fn status_build_diagnostic(&self, message: &Diagnostic) {
        // _ = self.progress.unbounded_send(BuildUpdateProgress {
        //     stage: Stage::Compiling,
        //     update: UpdateStage::AddMessage(message.clone().into()),
        //     platform: self.platform(),
        // });
    }

    pub(crate) fn status_build_message(&self, line: String) {
        // _ = self.progress.unbounded_send(BuildUpdateProgress {
        //     platform: self.platform(),
        //     stage: Stage::Compiling,
        //     update: UpdateStage::AddMessage(BuildMessage {
        //         level: Level::DEBUG,
        //         message: MessageType::Text(line),
        //         source: MessageSource::Build,
        //     }),
        // });
    }

    pub(crate) fn status_build_progress(&self, build_progress: f64) {
        _ = self.progress.unbounded_send(BuildUpdateProgress {
            platform: self.platform(),
            stage: Stage::Compiling,
            update: UpdateStage::SetProgress((build_progress).clamp(0.0, 1.00)),
        });
    }

    pub(crate) fn status_starting_build(&self) {
        _ = self.progress.unbounded_send(BuildUpdateProgress {
            stage: Stage::Compiling,
            update: UpdateStage::Start,
            platform: self.platform(),
        });
    }

    /// Try to get the unit graph for the crate. This is a nightly only feature which may not be available with the current version of rustc the user has installed.
    pub(crate) async fn get_unit_count(&self) -> Option<usize> {
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
            .args(self.build_arguments())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let output_text = String::from_utf8(output.stdout).ok()?;
        let graph: UnitGraph = serde_json::from_str(&output_text).ok()?;

        Some(graph.units.len())
    }

    /// Get an estimate of the number of units in the crate. If nightly rustc is not available, this will return an estimate of the number of units in the crate based on cargo metadata.
    /// TODO: always use https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#unit-graph once it is stable
    pub(crate) async fn get_unit_count_estimate(&self) -> usize {
        // Try to get it from nightly
        self.get_unit_count().await.unwrap_or_else(|| {
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
        tracing::info!("🚩 Build completed: [{}]", self.krate.out_dir().display());

        _ = self.progress.unbounded_send(BuildUpdateProgress {
            platform: self.platform(),
            stage: Stage::Finished,
            update: UpdateStage::Start,
        });
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
}

pub(crate) type ProgressTx = UnboundedSender<BuildUpdateProgress>;
pub(crate) type ProgressRx = UnboundedReceiver<BuildUpdateProgress>;

#[derive(Default, Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Stage {
    #[default]
    Initializing = 0,
    InstallingWasmTooling = 1,
    Compiling = 2,
    OptimizingWasm = 3,
    OptimizingAssets = 4,
    Finished = 5,
}

impl Deref for Stage {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Stage::Initializing => "Initializing",
            Stage::InstallingWasmTooling => "Installing Wasm Tooling",
            Stage::Compiling => "Compiling",
            Stage::OptimizingWasm => "Optimizing Wasm",
            Stage::OptimizingAssets => "Optimizing Assets",
            Stage::Finished => "Finished",
        }
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deref())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BuildUpdateProgress {
    pub(crate) stage: Stage,
    pub(crate) update: UpdateStage,
    pub(crate) platform: Platform,
}

impl BuildUpdateProgress {
    pub(crate) fn to_std_out(&self) {
        match &self.update {
            UpdateStage::Start => println!("--- {} ---", self.stage),
            UpdateStage::SetProgress(progress) => {
                println!("Build progress {:0.0}%", progress * 100.0);
            }
            UpdateStage::Failed(message) => {
                println!("Build failed: {}", message);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UpdateStage {
    Start,
    SetProgress(f64),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BuildMessage {
    pub(crate) level: Level,
    pub(crate) message: MessageType,
    pub(crate) source: MessageSource,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MessageType {
    Cargo(Diagnostic),
    Text(String),
}

/// Represents the source of where a message came from.
///
/// The CLI will render a prefix according to the message type
/// but this prefix, [`MessageSource::to_string()`] shouldn't be used if a strict message source is required.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MessageSource {
    /// Represents any message from the running application. Renders `[app]`
    App,

    /// Represents any generic message from the CLI. Renders `[dev]`
    ///
    /// Usage of Tracing inside of the CLI will be routed to this type.
    Dev,

    /// Represents a message from the build process. Renders `[bld]`
    ///
    /// This is anything emitted from a build process such as cargo and optimizations.
    Build,
}

impl Display for MessageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::App => write!(f, "app"),
            Self::Dev => write!(f, "dev"),
            Self::Build => write!(f, "bld"),
        }
    }
}

impl From<Diagnostic> for BuildMessage {
    fn from(message: Diagnostic) -> Self {
        Self {
            level: match message.level {
                cargo_metadata::diagnostic::DiagnosticLevel::Ice
                | cargo_metadata::diagnostic::DiagnosticLevel::FailureNote
                | cargo_metadata::diagnostic::DiagnosticLevel::Error => Level::ERROR,
                cargo_metadata::diagnostic::DiagnosticLevel::Warning => Level::WARN,
                cargo_metadata::diagnostic::DiagnosticLevel::Note => Level::INFO,
                cargo_metadata::diagnostic::DiagnosticLevel::Help => Level::DEBUG,
                _ => Level::DEBUG,
            },
            source: MessageSource::Build,
            message: MessageType::Cargo(message),
        }
    }
}

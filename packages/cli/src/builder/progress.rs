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
    /// Run `cargo`, returning the location of the final exectuable
    ///
    /// todo: add some stats here, like timing reports, crate-graph optimizations, etc
    pub(crate) async fn build_cargo(&self) -> anyhow::Result<PathBuf> {
        // Extract the unit count of the crate graph so build_cargo has more accurate data
        let crate_count = self.get_unit_count_estimate().await;

        _ = self.progress.unbounded_send(UpdateBuildProgress {
            stage: Stage::Compiling,
            update: UpdateStage::Start,
            platform: self.platform(),
        });

        let mut cmd = Command::new("cargo");

        cmd.arg("rustc")
            .envs(
                self.custom_target_dir
                    .as_ref()
                    .map(|dir| ("CARGO_TARGET_DIR", dir)),
            )
            .current_dir(self.krate.crate_dir())
            .arg("--message-format")
            .arg("json-diagnostic-rendered-ansi")
            .args(&self.build_arguments())
            .arg("--")
            .args(self.rust_flags.clone());

        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn cargo build")?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let stdout = tokio::io::BufReader::new(stdout);
        let stderr = tokio::io::BufReader::new(stderr);
        let mut output_location = None;

        let mut stdout = stdout.lines();
        let mut stderr = stderr.lines();
        let mut units_compiled = 0;
        let mut errors = Vec::new();

        loop {
            let line = tokio::select! {
                line = stdout.next_line() => line,
                line = stderr.next_line() => line,
            };

            let Some(line) = line? else {
                break;
            };

            let mut deserializer = serde_json::Deserializer::from_str(line.trim());
            deserializer.disable_recursion_limit();

            let message =
                Message::deserialize(&mut deserializer).unwrap_or(Message::TextLine(line));

            match message {
                Message::CompilerMessage(msg) => {
                    let message = msg.message;
                    _ = self.progress.unbounded_send(UpdateBuildProgress {
                        stage: Stage::Compiling,
                        update: UpdateStage::AddMessage(message.clone().into()),
                        platform: self.platform(),
                    });
                    const WARNING_LEVELS: &[cargo_metadata::diagnostic::DiagnosticLevel] = &[
                        cargo_metadata::diagnostic::DiagnosticLevel::Help,
                        cargo_metadata::diagnostic::DiagnosticLevel::Note,
                        cargo_metadata::diagnostic::DiagnosticLevel::Warning,
                        cargo_metadata::diagnostic::DiagnosticLevel::Error,
                        cargo_metadata::diagnostic::DiagnosticLevel::FailureNote,
                        cargo_metadata::diagnostic::DiagnosticLevel::Ice,
                    ];
                    const FATAL_LEVELS: &[cargo_metadata::diagnostic::DiagnosticLevel] = &[
                        cargo_metadata::diagnostic::DiagnosticLevel::Error,
                        cargo_metadata::diagnostic::DiagnosticLevel::FailureNote,
                        cargo_metadata::diagnostic::DiagnosticLevel::Ice,
                    ];
                    if WARNING_LEVELS.contains(&message.level) {
                        if let Some(rendered) = message.rendered {
                            errors.push(rendered);
                        }
                    }
                    if FATAL_LEVELS.contains(&message.level) {
                        return Err(anyhow::anyhow!(errors.join("\n")));
                    }
                }
                Message::CompilerArtifact(artifact) => {
                    units_compiled += 1;
                    if let Some(executable) = artifact.executable {
                        output_location = Some(executable.into());
                    } else {
                        let build_progress = units_compiled as f64 / crate_count as f64;
                        _ = self.progress.unbounded_send(UpdateBuildProgress {
                            platform: self.platform(),
                            stage: Stage::Compiling,
                            update: UpdateStage::SetProgress((build_progress).clamp(0.0, 1.00)),
                        });
                    }
                }
                Message::BuildScriptExecuted(_) => {
                    units_compiled += 1;
                }
                Message::BuildFinished(finished) => {
                    if !finished.success {
                        return Err(anyhow::anyhow!("Build failed"));
                    }
                }
                Message::TextLine(line) => {
                    _ = self.progress.unbounded_send(UpdateBuildProgress {
                        platform: self.platform(),
                        stage: Stage::Compiling,
                        update: UpdateStage::AddMessage(BuildMessage {
                            level: Level::DEBUG,
                            message: MessageType::Text(line),
                            source: MessageSource::Build,
                        }),
                    });
                }
                _ => {
                    // Unknown message
                }
            }
        }

        output_location.context("Build did not return an executable")
    }

    /// Try to get the unit graph for the crate. This is a nightly only feature which may not be available with the current version of rustc the user has installed.
    async fn get_unit_count(&self) -> Option<usize> {
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

        _ = self.progress.unbounded_send(UpdateBuildProgress {
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

pub(crate) type ProgressTx = UnboundedSender<UpdateBuildProgress>;
pub(crate) type ProgressRx = UnboundedReceiver<UpdateBuildProgress>;

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
pub(crate) struct UpdateBuildProgress {
    pub(crate) stage: Stage,
    pub(crate) update: UpdateStage,
    pub(crate) platform: Platform,
}

impl UpdateBuildProgress {
    pub(crate) fn to_std_out(&self) {
        match &self.update {
            UpdateStage::Start => println!("--- {} ---", self.stage),
            UpdateStage::AddMessage(message) => match &message.message {
                MessageType::Cargo(message) => {
                    println!("{}", message.rendered.clone().unwrap_or_default());
                }
                MessageType::Text(message) => {
                    println!("{}", message);
                }
            },
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
    AddMessage(BuildMessage),
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

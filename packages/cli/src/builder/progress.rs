//! Report progress about the build to the user. We use channels to report progress back to the CLI.

use cargo_metadata::CompilerMessage;
use cargo_metadata::{diagnostic::Diagnostic, Message};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, Stdout};
use tracing::Level;

use crate::build::Build;

#[derive(Default)]
pub enum Stage {
    #[default]
    Initializing,
    InstallingWasmTooling,
    Compiling,
    OptimizingWasm,
    OptimizingAssets,
    Finished,
}

pub struct UpdateBuildProgress {
    pub stage: Stage,
    pub update: UpdateStage,
}

impl UpdateBuildProgress {
    pub fn to_std_out(&self) {
        match &self.update {
            UpdateStage::Start => match self.stage {
                Stage::Initializing => {
                    println!("--- Initializing ---");
                }
                Stage::InstallingWasmTooling => {
                    println!("--- Installing wasm tooling ---");
                }
                Stage::Compiling => {
                    println!("--- Compiling ---");
                }
                Stage::OptimizingWasm => {
                    println!("--- Optimizing wasm ---");
                }
                Stage::OptimizingAssets => {
                    println!("--- Optimizing assets ---");
                }
                Stage::Finished => {
                    println!("--- Finished ---");
                }
            },
            UpdateStage::AddMessage(message) => {
                println!("{}", message.message);
            }
            UpdateStage::SetProgress(progress) => {
                println!("Build progress {:0.0}%", progress * 100.0);
            }
        }
    }
}

pub enum UpdateStage {
    Start,
    AddMessage(BuildMessage),
    SetProgress(f64),
}

pub struct BuildMessage {
    pub level: Level,
    pub message: String,
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
            message: message.rendered.unwrap_or_default(),
        }
    }
}

pub(crate) async fn build_cargo(
    crate_count: usize,
    mut cmd: tokio::process::Command,
    progress: &mut UnboundedSender<UpdateBuildProgress>,
) -> anyhow::Result<CargoBuildResult> {
    _ = progress.start_send(UpdateBuildProgress {
        stage: Stage::Compiling,
        update: UpdateStage::Start,
    });

    let stdout = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .stderr
        .take()
        .unwrap();
    let reader = tokio::io::BufReader::new(stdout);
    let mut output_location = None;

    let mut lines = reader.lines();
    let mut crates_compiled = 0;
    while let Ok(Some(line)) = lines.next_line().await {
        let mut deserializer = serde_json::Deserializer::from_str(&line);
        deserializer.disable_recursion_limit();
        let message = Message::deserialize(&mut deserializer).unwrap_or(Message::TextLine(line));
        match message {
            Message::CompilerMessage(msg) => {
                let message = msg.message;
                _ = progress.start_send(UpdateBuildProgress {
                    stage: Stage::Compiling,
                    update: UpdateStage::AddMessage(message.clone().into()),
                });
                if message.level == cargo_metadata::diagnostic::DiagnosticLevel::FailureNote {
                    return {
                        Err(anyhow::anyhow!(message
                            .rendered
                            .unwrap_or("Unknown".into())))
                    };
                }
            }
            Message::CompilerArtifact(artifact) => {
                crates_compiled += 1;
                if let Some(executable) = artifact.executable {
                    output_location = Some(executable.into());
                } else {
                    let build_progress = crates_compiled as f64 / crate_count as f64;
                    _ = progress.start_send(UpdateBuildProgress {
                        stage: Stage::Compiling,
                        update: UpdateStage::SetProgress((build_progress).clamp(0.0, 0.97)),
                    });
                }
            }
            Message::BuildFinished(finished) => {
                if !finished.success {
                    return Err(anyhow::anyhow!("Build failed"));
                }
            }
            Message::TextLine(line) => {
                _ = progress.start_send(UpdateBuildProgress {
                    stage: Stage::Compiling,
                    update: UpdateStage::AddMessage(BuildMessage {
                        level: Level::DEBUG,
                        message: line,
                    }),
                });
            }
            _ => {
                // Unknown message
            }
        }
    }

    Ok(CargoBuildResult { output_location })
}

pub(crate) struct CargoBuildResult {
    pub(crate) output_location: Option<PathBuf>,
}

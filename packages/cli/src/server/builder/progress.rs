//! Report progress about the build to the user. We use indicatif to report progress.

use cargo_metadata::{diagnostic::Diagnostic, Message};
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::time::Duration;

static PROGRESS_BARS: Lazy<indicatif::MultiProgress> = Lazy::new(indicatif::MultiProgress::new);

struct BuildProgress {
    progress_bar: Option<ProgressBar>,
}

impl BuildProgress {
    pub fn new() -> Self {
        let stdout = io::stdout().lock();

        let mut myself = Self { progress_bar: None };

        if stdout.is_terminal() {
            let mut pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::from_millis(200));
            pb = PROGRESS_BARS.add(pb);
            pb.set_style(
                ProgressStyle::with_template("{spinner:.dim.bold} {wide_msg}")
                    .unwrap()
                    .tick_chars("/|\\- "),
            );

            myself.progress_bar = Some(pb);
        }

        myself
    }

    /// Display a message to the user while the build is running
    pub fn display(&self, msg: impl ToString) {
        let msg = msg.to_string();
        if let Some(pb) = &self.progress_bar {
            pb.set_message(msg)
        } else {
            println!("{msg}");
        }
    }

    pub fn finish_with_message(&self, msg: impl ToString) {
        let msg = msg.to_string();
        if let Some(pb) = &self.progress_bar {
            pb.finish_with_message(msg)
        } else {
            println!("{msg}");
        }
    }
}

fn build_cargo(cmd: subprocess::Exec) -> anyhow::Result<CargoBuildResult> {
    let mut warning_messages: Vec<Diagnostic> = vec![];

    let output = BuildProgress::new();
    output.display("💼 Waiting to start building the project...");

    let stdout = cmd.detached().stream_stdout()?;
    let reader = std::io::BufReader::new(stdout);
    let mut output_location = None;

    for message in cargo_metadata::Message::parse_stream(reader) {
        match message.unwrap() {
            Message::CompilerMessage(msg) => {
                let message = msg.message;
                match message.level {
                    cargo_metadata::diagnostic::DiagnosticLevel::Error => {
                        return {
                            Err(anyhow::anyhow!(message
                                .rendered
                                .unwrap_or("Unknown".into())))
                        };
                    }
                    cargo_metadata::diagnostic::DiagnosticLevel::Warning => {
                        warning_messages.push(message.clone());
                    }
                    _ => {}
                }
            }
            Message::CompilerArtifact(artifact) => {
                output.display(format!("⚙ Compiling {} ", artifact.package_id));
                if let Some(executable) = artifact.executable {
                    output_location = Some(executable.into());
                }
            }
            Message::BuildScriptExecuted(script) => {
                let _package_id = script.package_id.to_string();
            }
            Message::BuildFinished(finished) => {
                if finished.success {
                    output.finish_with_message("👑 Build done.");
                } else {
                    output.finish_with_message("❌ Build failed.");
                    return Err(anyhow::anyhow!("Build failed"));
                }
            }
            _ => {
                // Unknown message
            }
        }
    }

    Ok(CargoBuildResult {
        warnings: warning_messages,
        output_location,
    })
}

struct CargoBuildResult {
    warnings: Vec<Diagnostic>,
    output_location: Option<PathBuf>,
}

use std::{time::Duration, io::Read};

use cargo_metadata::{diagnostic::Diagnostic, Message};
use indicatif::{ProgressBar, ProgressStyle};

use crate::Result;

pub fn pretty_build_output(stdout: impl Read) -> Result<()> {
    let mut warning_messages: Vec<Diagnostic> = vec![];

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(200));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.dim.bold} {wide_msg}")
            .unwrap()
            .tick_chars("/|\\- "),
    );
    pb.set_message("💼 Waiting to start build the project...");

    struct StopSpinOnDrop(ProgressBar);

    impl Drop for StopSpinOnDrop {
        fn drop(&mut self) {
            self.0.finish_and_clear();
        }
    }

    StopSpinOnDrop(pb.clone());

    let reader = std::io::BufReader::new(stdout);

    for message in cargo_metadata::Message::parse_stream(reader) {
        match message.unwrap() {
            Message::CompilerMessage(msg) => {
                let message = msg.message;
                match message.level {
                    cargo_metadata::diagnostic::DiagnosticLevel::Error => {
                        return Err(crate::Error::BuildFailed(message.to_string()));
                    }
                    cargo_metadata::diagnostic::DiagnosticLevel::Warning => {
                        warning_messages.push(message.clone());
                    }
                    _ => {}
                }
            }
            Message::CompilerArtifact(artifact) => {
                pb.set_message(format!("Compiling {} ", artifact.package_id));
                pb.tick();
            }
            Message::BuildScriptExecuted(script) => {
                let _package_id = script.package_id.to_string();
            }
            Message::BuildFinished(finished) => {
                if !finished.success {
                    std::process::exit(1);
                }
            }
            _ => (), // Unknown message
        }
    }

    for warning in warning_messages {
        log::warn!("{}", warning.message);
    }

    Ok(())
}

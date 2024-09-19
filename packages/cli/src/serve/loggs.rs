use std::{collections::HashMap, fmt::Display, io::stdout};

use super::*;
use crate::{BuildMessage, TraceMsg};
use crossterm::{
    cursor::Show,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
    TerminalOptions, Viewport,
};

/// Our console has "special" messages that get better rendering.
///
/// We want to display them differently since they have their own state and are rendered differently.
pub enum ConsoleMessage {
    Log(TraceMsg),
    OnngoingBuild { stage: Stage, progress: f64 },
    BuildReady,
}

#[derive(Default, Debug, PartialEq)]
pub struct ActiveBuild {
    pub stage: Stage,
    pub progress: f64,
    pub failed: Option<String>,
}

impl ActiveBuild {
    // fn update(&mut self, update: BuildUpdateProgress) {
    //     match update.update {
    //         UpdateStage::Start => {
    //             // If we are already past the stage, don't roll back, but allow a fresh build to update.
    //             if self.stage > update.stage && self.stage < Stage::Finished {
    //                 return;
    //             }
    //             self.stage = update.stage;
    //             self.progress = 0.0;
    //             self.failed = None;
    //         }
    //         UpdateStage::SetProgress(progress) => {
    //             self.progress = progress;
    //         }
    //         UpdateStage::Failed(failed) => {
    //             self.stage = Stage::Finished;
    //             self.failed = Some(failed.clone());
    //         }
    //     }
    // }

    pub fn make_spans(&self, area: Rect) -> Vec<Span> {
        let mut spans = Vec::new();

        let message = match self.stage {
            Stage::Initializing => "Initializing...",
            Stage::InstallingWasmTooling => "Configuring...",
            Stage::Compiling => "Compiling...",
            Stage::OptimizingWasm => "Optimizing...",
            Stage::OptimizingAssets => "Copying Assets...",
            Stage::Finished => "Build finished! 🎉 ",
        };

        let progress = format!(" {}%", (self.progress * 100.0) as u8);

        if area.width >= self.max_layout_size() {
            match self.stage {
                Stage::Finished => spans.push(Span::from(message).light_yellow()),
                _ => spans.push(Span::from(message).light_yellow()),
            }

            if self.stage != Stage::Finished {
                spans.push(Span::from(progress).white());
            }
        } else {
            spans.push(Span::from(progress).white());
        }

        spans
    }

    pub fn max_layout_size(&self) -> u16 {
        let progress_size = 4;
        let stage_size = self.stage.to_string().len() as u16;
        let brace_size = 2;

        progress_size + stage_size + brace_size
    }
}

impl PartialOrd for ActiveBuild {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            self.stage
                .cmp(&other.stage)
                .then(self.progress.partial_cmp(&other.progress).unwrap()),
        )
    }
}

pub fn set_fix_term_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        _ = disable_raw_mode();
        let mut stdout = stdout();
        _ = stdout.execute(LeaveAlternateScreen);
        _ = stdout.execute(Show);
        original_hook(info);
    }));
}

// todo: re-enable
#[allow(unused)]
async fn rustc_version() -> String {
    tokio::process::Command::new("rustc")
        .arg("--version")
        .output()
        .await
        .ok()
        .map(|o| o.stdout)
        .and_then(|o| {
            let out = String::from_utf8(o).unwrap();
            out.split_ascii_whitespace().nth(1).map(|v| v.to_string())
        })
        .unwrap_or_else(|| "<unknown>".to_string())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LogSource {
    Internal,
    Target(Platform),
}

impl Display for LogSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogSource::Internal => write!(f, "CLI"),
            LogSource::Target(platform) => write!(f, "{platform}"),
        }
    }
}

impl From<Platform> for LogSource {
    fn from(platform: Platform) -> Self {
        LogSource::Target(platform)
    }
}

#[derive(Default)]
pub(crate) struct BuildProgress {
    pub internal_logs: Vec<BuildMessage>,
    pub current_builds: HashMap<Platform, ActiveBuild>,
}

impl BuildProgress {
    pub fn progress(&self) -> f64 {
        self.current_builds
            .values()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|build| match build.stage {
                Stage::Initializing => 0.0,
                Stage::InstallingWasmTooling => 0.0,
                Stage::Compiling => build.progress,
                Stage::OptimizingWasm | Stage::OptimizingAssets | Stage::Finished => 1.0,
            })
            .unwrap_or_default()
    }
}

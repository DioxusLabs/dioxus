use cargo_metadata::CompilerMessage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use cargo_metadata;

/// The structured output for the CLI
///
/// This is designed such that third party tools can reliably consume the output of the CLI when
/// outputting json.
///
/// Not every log outputted will be parsable, but all structured logs should be.
///
/// This means the debug format of this log needs to be parsable json, not the default debug format.
///
/// We guarantee that the last line of the command represents the success of the command, such that
/// tools can simply parse the last line of the output.
///
/// There might be intermediate lines that are parseable as structured logs (which you can put here)
/// but they are not guaranteed to be, such that we can provide better error messages for the user.
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone)]
pub enum StructuredOutput {
    BuildFinished { path: PathBuf },
    BuildUpdate { stage: BuildStage },
    CargoOutput { message: CompilerMessage },
    BundleOutput { bundles: Vec<PathBuf> },
    HtmlTranslate { html: String },
    Success,
    Error { message: String },
}

impl std::fmt::Debug for StructuredOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_e| std::fmt::Error)?)
    }
}

/// The current stage of the ongoing build
///
/// This is a perma-unstable interface that is subject to change at any time.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BuildStage {
    Initializing,
    Starting {
        crate_count: usize,
        is_server: bool,
    },
    InstallingTooling,
    Compiling {
        is_server: bool,
        current: usize,
        total: usize,
        krate: String,
    },
    RunningBindgen,
    OptimizingWasm,
    PrerenderingRoutes,
    CopyingAssets {
        current: usize,
        total: usize,
        path: PathBuf,
    },
    Bundling,
    RunningGradle,
    Success,
    Failed,
    Aborted,
    Restarting,
}

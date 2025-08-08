use cargo_metadata::{diagnostic::Diagnostic, CompilerMessage};
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
    BuildsFinished {
        client: PathBuf,
        server: Option<PathBuf>,
    },
    BuildFinished {
        path: PathBuf,
    },
    BuildUpdate {
        stage: BuildStage,
    },
    CargoOutput {
        message: CompilerMessage,
    },
    RustcOutput {
        message: Diagnostic,
    },
    BundleOutput {
        bundles: Vec<PathBuf>,
    },
    HtmlTranslate {
        html: String,
    },

    Success,
    ExitRequested,
    Error {
        message: String,
    },
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
        patch: bool,
    },
    InstallingTooling,
    Compiling {
        current: usize,
        total: usize,
        krate: String,
    },
    RunningBindgen,
    SplittingBundle,
    OptimizingWasm,
    Linking,
    Hotpatching,
    ExtractingAssets,
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
    CompressingAssets,
    Prerendering,
}

impl BuildStage {
    /// Returns the identifier for this stage
    pub fn identifier(&self) -> &'static str {
        match self {
            BuildStage::Initializing => "initializing",
            BuildStage::Starting { .. } => "starting",
            BuildStage::InstallingTooling => "installing_tooling",
            BuildStage::Compiling { .. } => "compiling",
            BuildStage::RunningBindgen => "running_bindgen",
            BuildStage::SplittingBundle => "splitting_bundle",
            BuildStage::OptimizingWasm => "optimizing_wasm",
            BuildStage::Linking => "linking",
            BuildStage::Hotpatching => "hotpatching",
            BuildStage::ExtractingAssets => "extracting_assets",
            BuildStage::CopyingAssets { .. } => "copying_assets",
            BuildStage::Bundling => "bundling",
            BuildStage::RunningGradle => "running_gradle",
            BuildStage::Success => "success",
            BuildStage::Failed => "failed",
            BuildStage::Aborted => "aborted",
            BuildStage::Restarting => "restarting",
            BuildStage::CompressingAssets => "compressing_assets",
            BuildStage::Prerendering => "prerendering",
        }
    }
}

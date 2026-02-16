use cargo_metadata::{diagnostic::Diagnostic, CompilerMessage};
use manganis_core::BundledAsset;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashSet, path::PathBuf};
use subsecond_types::JumpTable;

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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum StructuredOutput {
    BuildsFinished {
        client: StructuredBuildArtifacts,
        server: Option<StructuredBuildArtifacts>,
    },
    PrintCargoArgs {
        args: Vec<String>,
        env: Vec<(Cow<'static, str>, String)>,
    },
    BuildFinished {
        artifacts: StructuredBuildArtifacts,
    },
    BuildUpdate {
        stage: BuildStage,
    },
    Hotpatch {
        jump_table: JumpTable,
        artifacts: StructuredBuildArtifacts,
    },
    CargoOutput {
        message: CompilerMessage,
    },
    RustcOutput {
        message: Diagnostic,
    },
    BundleOutput {
        bundles: Vec<PathBuf>,
        client: StructuredBuildArtifacts,
        server: Option<StructuredBuildArtifacts>,
    },
    HtmlTranslate {
        html: String,
    },
    Success,
    Error {
        message: String,
    },
}

impl std::fmt::Display for StructuredOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_e| std::fmt::Error)?)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredBuildArtifacts {
    pub path: PathBuf,
    pub exe: PathBuf,
    pub rustc_args: Vec<String>,
    pub rustc_envs: Vec<(String, String)>,
    pub link_args: Vec<String>,
    pub assets: HashSet<BundledAsset>, // the serialized asset manifest
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
    CompilingNativePlugins {
        detail: String,
    },
    CodeSigning,
    Success,
    Failed,
    Aborted,
    Restarting,
    CompressingAssets,
    Prerendering,
}

use crate::Result;
use std::path::PathBuf;

pub mod wasm;

/// Represents a file's type.
pub enum FileType {
    JavaScript,
    Css,
    // SASS & SCSS
    SassType,
    Wasm,
}

/// Represents a File on the device's storage system.
pub struct File {
    /// The name of the file.
    name: String,
    /// The path to the file.
    path: PathBuf,
    /// The file's type.
    file_type: FileType,
}

/// Represents a pipeline with it's own config and steps.
pub struct Pipeline {
    config: PipelineConfig,
    steps: Vec<Box<dyn PipelineStep>>,
}

impl Pipeline {
    /// Build a new pipeline.
    pub fn new(config: PipelineConfig) -> Self {
        // Collect all input files

        // Create config struct

        // Return self
        Self {
            config,
            steps: Vec::new(),
        }
    }

    /// Add a step to the pipeline.
    /// Steps run in the order they are added.
    pub fn with_step(&mut self, step: Box<dyn PipelineStep>) {
        self.steps.push(step);
    }

    /// Run the pipeline and all steps with it.
    pub fn run(mut self) -> Result<()> {
        for mut step in self.steps {
            step.run(&mut self.config)?;
        }

        Ok(())
    }
}

/// Configures the pipeline with the information it needs to complete.
pub struct PipelineConfig {
    /// Information related to the pipeline's target crate.
    crate_info: CrateInfo,
    /// Information related to how the pipeline should build the target crate.
    build_config: BuildConfig,
    /// Represents the raw source files.
    input_files: Vec<File>,
    /// Represents either a completed or processed artifact from the pipeline.
    output_files: Vec<File>,
}

impl PipelineConfig {
    /// Create a new PipelineConfig
    pub fn new(crate_info: CrateInfo, build_config: BuildConfig) -> Self {
        Self {
            crate_info,
            build_config,
            input_files: Vec::new(),
            output_files: Vec::new(),
        }
    }
}

/// Represents information about a crate.
pub struct CrateInfo {
    /// If the crate is in a workspace this value will point to the workspace root.
    workspace_path: Option<PathBuf>,
    /// The path to the crate itself.
    path: PathBuf,
    /// The name of the crate.
    name: String,
}

impl CrateInfo {
    /// Creates a new CrateInfo
    pub fn new(workspace_path: Option<PathBuf>, path: PathBuf, name: String) -> Self {
        Self {
            workspace_path,
            path,
            name,
        }
    }
}

/// Describes how the pipeline should build the target crate.
pub struct BuildConfig {
    /// Whether the pipeline should run for release mode.
    release: bool,
    /// Whether the pipeline should emit more verbose information.
    verbose: bool,
    /// The features the pipeline should build for.
    features: Vec<String>,
}

impl BuildConfig {
    /// Create a new BuildConfig
    pub fn new(release: bool, verbose: bool, features: Vec<String>) -> Self {
        Self {
            release,
            verbose,
            features,
        }
    }
}

/// Represents a step in the pipeline.
pub trait PipelineStep {
    fn run(&mut self, config: &mut PipelineConfig) -> Result<()>;
}

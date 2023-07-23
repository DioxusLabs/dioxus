use self::util::{File, OutputMessage};
use crate::{pipeline::util::MessageSeverity, Error, ProgressSpinner, Result};
use fs_extra::{dir::CopyOptions as DirCopyOptions, file::CopyOptions as FileCopyOptions};
use std::path::PathBuf;

pub mod index_file;
pub mod minify;
pub mod pull_assets;
pub mod sass;
pub mod util;
pub mod wasm_build;
pub mod wasm_opt;
pub mod web_out;

/// Represents a pipeline with it's own config and steps.
pub struct Pipeline {
    config: PipelineContext,
    steps: Vec<Box<dyn PipelineStep>>,
}

impl Pipeline {
    /// Build a new pipeline.
    pub fn new(config: PipelineContext) -> Self {
        Self {
            config,
            steps: Vec::new(),
        }
    }

    /// Add a step to the pipeline.
    /// Steps run in the order they are added.
    pub fn with_step(mut self, step: Box<dyn PipelineStep>) -> Self {
        self.steps.push(step);
        self
    }

    /// Run the pipeline and all steps with it.
    pub fn run(mut self) -> Result<()> {
        // Benchmarking
        let time_started = std::time::Instant::now();

        // Create staging
        self.config.create_fresh_staging()?;

        // Collect src input files
        let mut files = util::from_dir(PathBuf::from("./src"))?;
        self.config.raw_files.append(&mut files);

        // Sort steps by priority
        self.steps
            .sort_unstable_by(|a, b| a.priority().cmp(&b.priority()));

        let pb = ProgressSpinner::new("Starting pipeline");
        self.config.progress_spinner = Some(pb.clone());

        // In the future we could add multithreaded support
        for step in self.steps.iter_mut() {
            step.run(&mut self.config)?;
        }

        // Everything is finished.
        for step in self.steps.iter_mut() {
            step.pipeline_finished(&mut self.config)?;
        }

        // Delete staging
        self.config.delete_staging()?;

        // Final messages
        pb.done_and_clear();
        for msg in self.config.output_messages.iter() {
            match msg.severity {
                MessageSeverity::Info => log::info!("{}", msg.message),
                MessageSeverity::Warn => log::warn!("{}", msg.message),
            }
        }

        // End benchmark
        let elapsed = time_started.elapsed();
        let seconds = elapsed.as_secs_f32();
        log::info!("Pipeline finished successfully in {:.2}s", seconds);

        Ok(())
    }
}

/// Acts as the single source of information for all pipeline steps.
pub struct PipelineContext {
    /// Information related to the pipeline's target crate.
    crate_info: CrateInfo,
    /// Information related to how the pipeline should build the target crate.
    build_config: BuildConfig,
    /// Represents raw source files.
    raw_files: Vec<File>,
    /// Represents processed files.
    processed_files: Vec<File>,
    /// Represents a copy of the pipeline's progress spinner.
    progress_spinner: Option<ProgressSpinner>,
    /// A list of messages to emit when the pipeline finishes
    output_messages: Vec<OutputMessage>,
}

impl PipelineContext {
    const STAGING_PATH: &str = "./staging";

    /// Create a new PipelineContext
    pub fn new(crate_info: CrateInfo, build_config: BuildConfig) -> Self {
        Self {
            crate_info,
            build_config,
            raw_files: Vec::new(),
            processed_files: Vec::new(),
            progress_spinner: None,
            output_messages: Vec::new(),
        }
    }

    pub fn add_output_message(&mut self, message: OutputMessage) {
        self.output_messages.push(message);
    }

    pub fn set_message<S: ToString>(&self, message: S) {
        if let Some(pb) = &self.progress_spinner {
            pb.set_message(message);
        }
    }

    /// Creates an empty staging directory, deleting any existing ones.
    fn create_fresh_staging(&self) -> Result<()> {
        self.delete_staging()?;
        std::fs::create_dir(self.crate_info.path.join(Self::STAGING_PATH))?;
        Ok(())
    }

    /// Deletes the staging directory if it exists.
    fn delete_staging(&self) -> Result<()> {
        let staging_path = self.crate_info.path.join(Self::STAGING_PATH);
        if staging_path.exists() {
            std::fs::remove_dir_all(staging_path)?;
        }
        Ok(())
    }

    /// Returns a [`Pathbuf`] to the staging directory.
    pub fn staging_path(&self) -> PathBuf {
        self.crate_info.path.join(Self::STAGING_PATH)
    }

    /// Moves a single file to the staging directory.
    pub fn copy_file_to_staging(&self, file_path: PathBuf) -> Result<PathBuf> {
        let file_name = if let Some(file_path) = file_path.file_name() {
            file_path
        } else {
            return Err(Error::ParseError("Failed to get file name.".to_string()));
        };

        // Get full path
        let full_path = self
            .crate_info
            .path
            .join(Self::STAGING_PATH)
            .join(file_name);

        // Copy file
        fs_extra::file::copy(
            file_path,
            full_path.clone(),
            &FileCopyOptions::new().overwrite(true),
        )
        .map_err(|e| Error::CustomError(e.to_string()))?;

        Ok(full_path)
    }

    /// Copies everything from the staging directory to the specified directory.
    pub fn copy_staging_to_dir(&self, dir_path: PathBuf) -> Result<()> {
        fs_extra::dir::copy(
            self.crate_info.path.join(Self::STAGING_PATH),
            dir_path,
            &DirCopyOptions::new().overwrite(true).content_only(true),
        )
        .map_err(|e| Error::CustomError(e.to_string()))?;
        Ok(())
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

    /// Automagically get the crate config from ``Cargo.toml``.
    pub fn from_toml(target_bin: Option<PathBuf>) -> Result<Self> {
        let mut workspace_path = PathBuf::from("../");
        let mut crate_path = PathBuf::from("./");

        // If target bin, we should already be in a workspace
        if let Some(bin) = target_bin {
            workspace_path = PathBuf::from("./");
            crate_path = bin
        }

        // Check if workspace is actually a workspace
        let workspace_path = if let Ok(manifest) =
            cargo_toml::Manifest::from_path(workspace_path.join("Cargo.toml"))
        {
            if manifest.workspace.is_some() {
                Some(workspace_path)
            } else {
                None
            }
        } else {
            None
        };

        // Get target crate's Cargo.toml
        let manifest = cargo_toml::Manifest::from_path(crate_path.join("Cargo.toml"))
            .map_err(|e| Error::CargoError(e.to_string()))?;

        // Get package name
        let name = if let Some(package) = manifest.package {
            package.name
        } else {
            return Err(Error::CargoError(
                "No buildable crates found. Are you running this from the correct path?\nIf this is a workspace, use the --bin flag.".to_string(),
            ));
        };

        Ok(Self::new(workspace_path, crate_path, name))
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

/// Represents the priority of the step: How important it is to run first vs last.
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub enum StepPriority {
    /// Ideal for steps that pull in additional assets that need to be processed.
    Highest,
    /// Ideal for steps that generate more files that need to be processed.
    High,
    /// Ideal for steps that process existing files and convert them into new files.
    Medium,
    /// Ideal for steps that do final touches, bundling, or similar.
    Low,
    /// Ideal for steps that generate the final output.
    Lowest,
}

/// Represents a step in the pipeline.
pub trait PipelineStep {
    /// Called when the step needs to run.
    fn run(&mut self, config: &mut PipelineContext) -> Result<()>;
    /// Called when the entire pipeline is finished.
    fn pipeline_finished(&mut self, config: &mut PipelineContext) -> Result<()>;
    /// Gets the step's priority.
    fn priority(&self) -> StepPriority;
}

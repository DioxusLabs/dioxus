use crate::Result;
use std::path::PathBuf;

pub mod wasm;

pub enum FileType {
    JavaScript,
    Css,
    // SASS & SCSS
    SassType,
}

pub struct File {
    pub name: String,
    pub path: PathBuf,
    pub file_type: FileType,
}

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

pub struct PipelineConfig {
    crate_path: PathBuf,
    release: bool,
    verbose: bool,
    features: Vec<String>,
    input_files: Vec<File>,
    output_files: Vec<File>,
}

impl PipelineConfig {
    pub fn new(crate_path: PathBuf, release: bool, verbose: bool, features: Vec<String>) -> Self {
        Self {
            crate_path,
            release,
            verbose,
            features,
            input_files: Vec::new(),
            output_files: Vec::new(),
        }
    }
}

pub trait PipelineStep {
    fn run(&mut self, config: &mut PipelineConfig) -> Result<()>;
}

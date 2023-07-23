use std::path::PathBuf;

use super::{PipelineContext, PipelineStep};

const DIST_PATH: &str = "./dist";

/// Outputs build artifacts into a useable state for the web target.
pub struct WebOut {}

impl WebOut {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for WebOut {
    fn run(&mut self, config: &mut PipelineContext) -> crate::Result<()> {
        config.set_message("Outputting web files");
        
        // Create dist folder
        let dist_path = PathBuf::from(DIST_PATH);
        if dist_path.exists() {
            std::fs::remove_dir_all(dist_path.clone())?;
        }
        std::fs::create_dir_all(dist_path.clone())?;

        // Move staging to dist
        config.copy_staging_to_dir(dist_path)?;

        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::Lowest
    }
}

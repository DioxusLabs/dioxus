use std::path::PathBuf;

use super::{PipelineConfig, PipelineStep};

const DIST_PATH: &str = "./dist";

/// Outputs build artifacts into a useable state for the web target.
pub struct WebOut {}

impl WebOut {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for WebOut {
    fn run(&mut self, _config: &mut PipelineConfig) -> crate::Result<()> {
        Ok(())
    }

    fn pipeline_finished(&mut self, config: &mut PipelineConfig) -> crate::Result<()> {
        log::info!("Configuring for web output...");

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
        super::StepPriority::Low
    }
}

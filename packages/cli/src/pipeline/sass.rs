use super::{PipelineConfig, PipelineStep};
use crate::tools;

/// Generates CSS files from any SASS or SCSS files.
pub struct SassBuild {}

impl SassBuild {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for SassBuild {
    fn run(&mut self, config: &mut PipelineConfig) -> crate::Result<()> {
        log::info!("Building SASS/SCSS files");

        let sass = tools::Sass::get()?;

        Ok(())
    }

    fn pipeline_finished(&mut self, _config: &mut PipelineConfig) -> crate::Result<()> {
        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::High
    }
}

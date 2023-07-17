use super::{PipelineConfig, PipelineStep};
use crate::pipeline::util;

const PUBLIC_PATH: &str = "./public";
const ASSETS_PATH: &str = "./assets";

/// Pulls assets from directories other than ``src``: ``public`` & ``assets``.
pub struct PullAssets {}

impl PullAssets {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for PullAssets {
    fn run(&mut self, config: &mut PipelineConfig) -> crate::Result<()> {
        log::info!("Pulling additional assets");

        let mut public_files = util::from_dir(config.crate_info.path.join(PUBLIC_PATH))?;
        let mut assets_files = util::from_dir(config.crate_info.path.join(ASSETS_PATH))?;

        config.input_files.append(&mut public_files);
        config.input_files.append(&mut assets_files);

        Ok(())
    }

    fn pipeline_finished(&mut self, _config: &mut PipelineConfig) -> crate::Result<()> {
        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::High
    }
}

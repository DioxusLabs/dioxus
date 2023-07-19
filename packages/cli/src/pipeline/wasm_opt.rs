use std::fs;

use super::{PipelineConfig, PipelineStep};
use crate::{
    pipeline::util::{self, FileType},
    tools,
};

/// Optimizes wasm files.
pub struct WasmOpt {}

impl WasmOpt {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for WasmOpt {
    fn run(&mut self, config: &mut PipelineConfig) -> crate::Result<()> {
        if !config.build_config.release {
            return Ok(());
        }

        // Get wasm tool
        let opt = tools::WasmOpt::get()?;

        // Get wasm files in the staging folder
        let files = util::from_dir(config.staging_path())?;
        let mut bytes_init_total = 0;
        let mut bytes_final_total = 0;

        for file in files.iter() {
            // If file isn't wasm, skip it.
            if file.file_type != FileType::Wasm {
                continue;
            }
            // Add total bytes
            bytes_init_total += fs::metadata(&file.path)?.len();

            opt.run(file.path.clone(), file.path.clone())?;

            // Add final bytes
            bytes_final_total += fs::metadata(&file.path)?.len();
        }

        let kb_saved = (bytes_init_total - bytes_final_total) / 1000;
        log::info!("Optimizing WASM saved {}kb", kb_saved);
        Ok(())
    }

    fn pipeline_finished(&mut self, _config: &mut PipelineConfig) -> crate::Result<()> {
        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::Low
    }
}

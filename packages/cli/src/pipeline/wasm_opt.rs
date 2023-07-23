use std::fs;

use super::{
    util::{MessageSeverity, OutputMessage},
    PipelineContext, PipelineStep,
};
use crate::{pipeline::util::FileType, tools};

/// Optimizes wasm files.
pub struct WasmOpt {}

impl WasmOpt {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl PipelineStep for WasmOpt {
    fn run(&mut self, config: &mut PipelineContext) -> crate::Result<()> {
        if !config.build_config.release {
            return Ok(());
        }

        // Get wasm tool
        config.set_message("Installing wasm-opt");
        let opt = tools::WasmOpt::get()?;
        config.set_message("Optimizing wasm");

        let mut bytes_init_total = 0;
        let mut bytes_final_total = 0;

        for file in config.processed_files.iter() {
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
        let msg = format!("Optimizing wasm saved {}kb", kb_saved);
        config.add_output_message(OutputMessage::new(MessageSeverity::Info, msg));

        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::Low
    }
}

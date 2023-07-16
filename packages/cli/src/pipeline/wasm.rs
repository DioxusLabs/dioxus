use std::process::Command;

use crate::Error;

use super::{PipelineConfig, PipelineStep};

pub struct WasmBuild {}

impl PipelineStep for WasmBuild {
    fn run(&mut self, config: &mut PipelineConfig) -> crate::Result<()> {
        log::info!("Building wasm...");

        // Construct command
        let mut cmd = subprocess::Exec::cmd("cargo");

        cmd = cmd
            .cwd(config.crate_path.clone())
            .arg("build")
            .arg("--target")
            .arg("wasm32-unknown-unknown");

        if config.release {
            cmd = cmd.arg("--release");
        }

        if config.verbose {
            cmd = cmd.arg("--verbose");
        }

        // Run command
        //cmd.detached().map_err(|e| Error::BuildFailed(e.to_string()))?;

        log::info!("Finished building wasm.");
        Ok(())
    }
}

use crate::{pipeline::util, tools, Result};

use super::{PipelineConfig, PipelineStep};
use std::path::PathBuf;

const DEBUG_TARGET: &str = "target/wasm32-unknown-unknown/debug";
const RELEASE_TARGET: &str = "target/wasm32-unknown-unknown/release";
const STAGING_OUT: &str = "./bindgen_out";

pub struct WasmBuild {}

impl WasmBuild {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    /// Runs the wasm-bindgen CLI on the specified wasm file.
    pub fn run_bindgen(&self, config: &PipelineConfig, file: PathBuf) -> Result<PathBuf> {
        let out = config.staging_path().join(STAGING_OUT);
        let release = config.build_config.release;

        tools::Bindgen::get()?
            .debug(!release)
            .keep_debug(!release)
            .no_demangle(!release)
            .run(file, out.clone())?;

        Ok(out)
    }
}

impl PipelineStep for WasmBuild {
    fn run(&mut self, config: &mut PipelineConfig) -> crate::Result<()> {
        log::info!("Building wasm");

        // Construct command
        let mut cmd = subprocess::Exec::cmd("cargo")
            .cwd(config.crate_info.path.clone())
            .arg("build")
            .arg("--target")
            .arg("wasm32-unknown-unknown");

        // Set release
        if config.build_config.release {
            cmd = cmd.arg("--release");
        }

        // Set verbose
        if config.build_config.verbose {
            cmd = cmd.arg("--verbose");
        }

        // Set features
        if !config.build_config.features.is_empty() {
            cmd = cmd.arg("--features");
            cmd = cmd.arg(config.build_config.features.join(" "))
        }

        // Run command
        _ = cmd
            .join()
            .map_err(|e| crate::Error::BuildFailed(e.to_string()))?;

        // Build the path
        let mut wasm_out_path = PathBuf::new();

        // Get the full path to target
        if let Some(workspace_path) = &config.crate_info.workspace_path {
            wasm_out_path.push(workspace_path);
        } else {
            wasm_out_path.push(config.crate_info.path.clone());
        };

        // Get the target 'inner' path
        if config.build_config.release {
            wasm_out_path.push(RELEASE_TARGET);
        } else {
            wasm_out_path.push(DEBUG_TARGET);
        };

        // Get the final path to the built wasm file
        wasm_out_path.push(format!("{}.wasm", config.crate_info.name));

        // Run bindgen on the file
        let bindgen_out_path = self.run_bindgen(config, wasm_out_path)?;

        // Add all output files to config for further processing.
        let mut bindgen_files = util::from_dir(bindgen_out_path)?;
        config.files.append(&mut bindgen_files);

        Ok(())
    }

    fn pipeline_finished(&mut self, _config: &mut PipelineConfig) -> crate::Result<()> {
        Ok(())
    }

    fn priority(&self) -> super::StepPriority {
        super::StepPriority::High
    }
}

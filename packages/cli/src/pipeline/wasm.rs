use super::{PipelineConfig, PipelineStep};
use crate::pipeline::{File, FileType, util::pretty_build_output};
use std::path::PathBuf;

const DEBUG_TARGET: &str = "/target/wasm32-unknown-unknown/debug";
const RELEASE_TARGET: &str = "/target/wasm32-unknown-unknown/release";

pub struct WasmBuild {}

impl PipelineStep for WasmBuild {
    fn run(&mut self, config: &mut PipelineConfig) -> crate::Result<()> {
        log::info!("Building wasm...");

        // Construct command
        let mut cmd = subprocess::Exec::cmd("cargo")
            .cwd(config.crate_info.path.clone())
            .arg("build")
            .arg("--target")
            .arg("wasm32-unknown-unknown")
            .arg("--message-format=json");

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
        let cmd_stdout = cmd.detached().stream_stdout().map_err(|e| crate::Error::BuildFailed(e.to_string()))?;
        pretty_build_output(cmd_stdout)?;

        // Get the target 'inner' path
        // /target/{X}
        let target_path = if config.build_config.release {
            PathBuf::from(RELEASE_TARGET)
        } else {
            PathBuf::from(DEBUG_TARGET)
        };

        // Get the full path to target
        // {PATH}/target/{X}
        let target_path = if let Some(workspace_path) = &config.crate_info.workspace_path {
            workspace_path.join(target_path)
        } else {
            config.crate_info.path.join(target_path)
        };

        // Get the final path to the built wasm file
        // {PATH}/target/{X}/{CRATE_NAME}.wasm
        let wasm_out_path = target_path.join(format!("{}.wasm", config.crate_info.name));

        log::info!("{}", wasm_out_path.to_str().unwrap());

        // Create the file metadata
        let out_file = File {
            name: config.crate_info.name.clone(),
            path: wasm_out_path,
            file_type: FileType::Wasm,
        };

        // Push it to out files for later processing
        config.output_files.push(out_file);

        log::info!("Finished building wasm.");
        Ok(())
    }
}

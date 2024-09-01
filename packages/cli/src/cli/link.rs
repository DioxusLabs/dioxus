use std::{env::current_dir, path::PathBuf};

use serde::{Deserialize, Serialize};

/// The env var that will be set by the linker intercept cmd to indicate that we should act as a linker
pub const LINK_OUTPUT_ENV_VAR: &str = "dx-magic-link-file";

/// Should we act as a linker?
///
/// Just check if the magic env var is set
pub fn should_link() -> bool {
    std::env::var(LINK_OUTPUT_ENV_VAR).is_ok()
}

#[derive(Serialize, Deserialize)]
pub struct InterceptedArgs {
    pub work_dir: PathBuf,
    pub args: Vec<String>,
}

/// Write the incoming linker args to a file
///
/// The file will be given by the dx-magic-link-arg env var itself, so we use
/// it both for determining if we should act as a linker and the for the file name itself.
///
/// This will panic if it fails
pub fn dump_link_args() -> anyhow::Result<()> {
    let output = std::env::var(LINK_OUTPUT_ENV_VAR).expect("Missing env var with target file");

    // get the args and then dump them to the file
    let args: Vec<_> = std::env::args().collect();
    let escaped = serde_json::to_string(&InterceptedArgs {
        args,
        work_dir: current_dir().unwrap(),
    })
    .expect("Failed to escape env args");

    // write the file
    std::fs::write(output, escaped).expect("Failed to write output file");

    Ok(())
}

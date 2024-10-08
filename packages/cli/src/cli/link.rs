use std::{env::current_dir, path::PathBuf};

use serde::{Deserialize, Serialize};

/// The env var that will be set by the linker intercept cmd to indicate that we should act as a linker
pub(crate) const LINK_OUTPUT_ENV_VAR: &str = "dx-magic-link-file";

/// Should we write the input arguments to a file (aka act as a linker subprocess)?
///
/// Just check if the magic env var is set
pub(crate) fn should_dump_link_args() -> bool {
    std::env::var(LINK_OUTPUT_ENV_VAR).is_ok()
}

#[derive(Serialize, Deserialize)]
pub(crate) struct InterceptedArgs {
    pub(crate) work_dir: PathBuf,
    pub(crate) args: Vec<String>,
}

/// Write the incoming linker args to a file
///
/// The file will be given by the dx-magic-link-arg env var itself, so we use
/// it both for determining if we should act as a linker and the for the file name itself.
///
/// This will panic if it fails
///
/// hmmmmmmmm tbh I'd rather just pass the object files back and do the parsing here, but the interface
/// is nicer to just bounce back the args and let the host do the parsing/canonicalization
pub(crate) fn dump_link_args() -> anyhow::Result<()> {
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

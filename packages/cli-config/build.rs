use std::error::Error;
use vergen_gix::{Emitter, GixBuilder as GitBuilder};

// warn if the "read-config" feature is enabled, but the DIOXUS_CONFIG environment variable is not set
// This means that some library is trying to access the crate's configuration, but the dioxus CLI was not used to build the application.
fn main() -> Result<(), Box<dyn Error>> {
    Emitter::default()
        .add_instructions(&GitBuilder::all_git()?)?
        .emit()?;

    println!("cargo:rerun-if-env-changed=DIOXUS_CONFIG");
    let dioxus_config = std::env::var("DIOXUS_CONFIG");
    let built_with_dioxus = dioxus_config.is_ok();
    if cfg!(feature = "read-config") && !built_with_dioxus {
        println!("cargo:warning=A library is trying to access the crate's configuration, but the dioxus CLI was not used to build the application. Information about the Dioxus CLI is available at https://dioxuslabs.com/learn/0.5/CLI/installation");
    }
    Ok(())
}

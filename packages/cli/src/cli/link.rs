use crate::{Platform, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub enum LinkAction {
    BaseLink {
        strip: bool,
        platform: Platform,
        linker: PathBuf,
        incremental_dir: PathBuf,
    },
    ThinLink {
        platform: Platform,
        main_ptr: u64,
        patch_target: PathBuf,
        linker: PathBuf,
        incremental_dir: PathBuf,
    },
}

impl LinkAction {
    pub(crate) const ENV_VAR_NAME: &'static str = "dx_magic_link_file";

    /// Should we write the input arguments to a file (aka act as a linker subprocess)?
    ///
    /// Just check if the magic env var is set
    pub(crate) fn from_env() -> Option<Self> {
        std::env::var(Self::ENV_VAR_NAME)
            .ok()
            .map(|var| serde_json::from_str(&var).expect("Failed to parse magic env var"))
    }

    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Write the incoming linker args to a file
    ///
    /// The file will be given by the dx-magic-link-arg env var itself, so we use
    /// it both for determining if we should act as a linker and the for the file name itself.
    pub(crate) async fn run(self) -> Result<()> {
        let args = std::env::args().collect::<Vec<String>>();

        match self {
            // Run the system linker but (maybe) keep any unused sections.
            LinkAction::BaseLink {
                platform,
                incremental_dir,
                linker,
                strip,
            } => {
                // Make sure we *don't* dead-strip the binary so every library symbol still exists.
                //  This is required for thin linking to work correctly.
                let args = args
                    .into_iter()
                    .skip(1)
                    .filter(|arg| arg != "-Wl,-dead_strip" && !strip)
                    .collect::<Vec<String>>();

                // Persist the cache of incremental files
                cache_incrementals(
                    &incremental_dir.join("old"),
                    &incremental_dir.join("new"),
                    args.iter()
                        .filter(|arg| arg.ends_with(".o"))
                        .collect::<Vec<&String>>()
                        .as_ref(),
                );

                // Run ld with the args
                let res = Command::new(linker).args(args).output().await?;
                let err = String::from_utf8_lossy(&res.stderr);
            }

            // Run the linker but without rlibs
            LinkAction::ThinLink {
                linker,
                platform,
                patch_target,
                incremental_dir,
                main_ptr,
            } => {
                let index_of_out = args.iter().position(|arg| arg == "-o").unwrap();
                let out_file = args[index_of_out + 1].clone();
                let object_files: Vec<_> = args.iter().filter(|arg| arg.ends_with(".o")).collect();

                cache_incrementals(
                    &incremental_dir.join("old"),
                    &incremental_dir.join("new"),
                    object_files.as_ref(),
                );

                let res = Command::new("cc")
                    .args(object_files)
                    .arg("-dylib")
                    .arg("-undefined")
                    .arg("dynamic_lookup")
                    .arg("-Wl,-unexported_symbol,_main")
                    .arg("-arch")
                    .arg("arm64")
                    .arg("-dead_strip") // maybe?
                    .arg("-o")
                    .arg(&out_file)
                    // .stdout(Stdio::piped())
                    // .stderr(Stdio::piped())
                    .output()
                    .await?;

                // crate::build::attempt_partial_link(
                //     linker,
                //     incremental_dir.clone(),
                //     incremental_dir.join("old"),
                //     incremental_dir.join("new"),
                //     main_ptr,
                //     patch_target,
                //     out_file.clone().into(),
                // )
                // .await;
            }
        }

        Ok(())
    }
}

/// Move all previous object files to "incremental-old" and all new object files to "incremental-new"
fn cache_incrementals(old: &PathBuf, new: &PathBuf, object_files: &[&String]) {
    // Remove the old incremental-old directory if it exists
    _ = std::fs::remove_dir_all(&old);

    // Rename incremental-new to incremental-old if it exists. Faster than moving all the files
    _ = std::fs::rename(&new, &old);

    // Create the new incremental-new directory to place the outputs in
    std::fs::create_dir_all(&new).unwrap();

    // Now drop in all the new object files
    for o in object_files.iter() {
        if !o.ends_with(".rcgu.o") {
            continue;
        }

        let path = PathBuf::from(o);
        std::fs::copy(&path, new.join(path.file_name().unwrap())).unwrap();
    }
}

use crate::{Platform, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub enum LinkAction {
    LinkAndroid {
        linker: PathBuf,
        extra_flags: Vec<String>,
    },
    FatLink {
        platform: Platform,
    },
    ThinLink {
        platform: Platform,
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
    ///
    /// This will panic if it fails
    ///
    /// hmmmmmmmm tbh I'd rather just pass the object files back and do the parsing here, but the interface
    /// is nicer to just bounce back the args and let the host do the parsing/canonicalization
    pub(crate) async fn run(self) -> Result<()> {
        let args = std::env::args().collect::<Vec<String>>();

        match self {
            // Run the android linker passed to us via the env var
            LinkAction::LinkAndroid {
                linker,
                extra_flags,
            } => {
                let mut cmd = std::process::Command::new(linker);
                cmd.args(std::env::args().skip(1));
                cmd.args(extra_flags);
                cmd.stderr(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .status()
                    .expect("Failed to run android linker");
            }

            // Run the system linker but keep any unused sections
            LinkAction::FatLink { platform } => {
                let args = args
                    .into_iter()
                    .skip(1)
                    .filter(|arg| arg != "-Wl,-dead_strip")
                    .collect::<Vec<String>>();

                let object_files: Vec<_> = args.iter().filter(|arg| arg.ends_with(".o")).collect();
                cache_incrementals(object_files.as_ref());

                // Run ld with the args
                let res = Command::new("cc").args(args).output().await?;
                let err = String::from_utf8_lossy(&res.stderr);
            }

            // Run the linker but without rlibs
            LinkAction::ThinLink { platform } => {
                let index_of_out = args.iter().position(|arg| arg == "-o").unwrap();
                let out_file = args[index_of_out + 1].clone();
                let object_files: Vec<_> = args.iter().filter(|arg| arg.ends_with(".o")).collect();

                cache_incrementals(object_files.as_ref());

                let patch_target =
                    "/Users/jonkelley/Development/Tinkering/ipbp/target/hotreload/harness".into();

                let main_ptr = std::fs::read_to_string(workspace_root().join("harnessaddr.txt"))
                    .unwrap()
                    .parse()
                    .unwrap();

                crate::build::attempt_partial_link(main_ptr, patch_target, out_file.clone().into())
                    .await;
            }
        }

        Ok(())
    }
}

/// Move all previous object files to "incremental-old" and all new object files to "incremental-new"
fn cache_incrementals(object_files: &[&String]) {
    let old = workspace_root().join("data").join("incremental-old");
    let new = workspace_root().join("data").join("incremental-new");

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

fn system_linker(platform: Platform) -> &'static str {
    // match platform {
    //     Platform::MacOS => "ld",
    //     Platform::Windows => "ld",
    //     Platform::Linux => "ld",
    //     Platform::Ios => "ld",
    //     Platform::Android => "ld",
    //     Platform::Server => "ld",
    //     Platform::Liveview => "ld",
    // }
}

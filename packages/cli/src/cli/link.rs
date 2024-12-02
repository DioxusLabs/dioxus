use dioxus_cli_opt::AssetManifest;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub enum LinkAction {
    BuildAssetManifest {
        destination: PathBuf,
    },
    LinkAndroid {
        linker: PathBuf,
        extra_flags: Vec<String>,
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
    pub(crate) fn run(self) {
        match self {
            // Literally just run the android linker :)
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

            // Assemble an asset manifest by walking the object files being passed to us
            LinkAction::BuildAssetManifest { destination: dest } => {
                let mut args: Vec<_> = std::env::args().collect();
                let mut manifest = AssetManifest::default();

                // Handle command files, usually a windows thing.
                if let Some(command) = args.iter().find(|arg| arg.starts_with('@')).cloned() {
                    let path = command.trim().trim_start_matches('@');
                    let file_binary = std::fs::read(path).unwrap();

                    // This may be a utf-16le file. Let's try utf-8 first.
                    let content = String::from_utf8(file_binary.clone()).unwrap_or_else(|_| {
                        // Convert Vec<u8> to Vec<u16> to convert into a String
                        let binary_u16le: Vec<u16> = file_binary
                            .chunks_exact(2)
                            .map(|a| u16::from_le_bytes([a[0], a[1]]))
                            .collect();

                        String::from_utf16_lossy(&binary_u16le)
                    });

                    // Gather linker args, and reset the args to be just the linker args
                    args = content
                        .lines()
                        .map(|line| {
                            let line_parsed = line.to_string();
                            let line_parsed = line_parsed.trim_end_matches('"').to_string();
                            let line_parsed = line_parsed.trim_start_matches('"').to_string();
                            line_parsed
                        })
                        .collect();
                }

                // Parse through linker args for `.o` or `.rlib` files.
                for item in args {
                    if item.ends_with(".o") || item.ends_with(".rlib") {
                        let path_to_item = PathBuf::from(item);
                        if let Ok(path) = path_to_item.canonicalize() {
                            _ = manifest.add_from_object_path(&path);
                        }
                    }
                }

                let contents = serde_json::to_string(&manifest).expect("Failed to write manifest");
                std::fs::write(dest, contents).expect("Failed to write output file");
            }
        }
    }
}

use const_serialize::{deserialize_const, ConstVec};
use manganis::BundledAsset;
use object::{Object, ObjectSection, ReadCache};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Debug;
use std::fs;
use std::io::{Read, Seek, Write};
use std::ops::Range;
use std::path::PathBuf;
use std::process::Command;
use target_lexicon::Triple;

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
                let args: Vec<_> = std::env::args().collect();
                let mut obj_args = args.clone();
                let mut manifest = AssetReferences::default();

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
                    obj_args = content
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
                obj_args.retain(|item| item.ends_with(".o") || item.ends_with(".rlib"));
                for item in obj_args {
                    let path_to_item = PathBuf::from(item);
                    if let Ok(path) = path_to_item.canonicalize() {
                        _ = manifest.add_from_object_path(&path);
                    }
                }

                // let contents = serde_json::to_string(&manifest).expect("Failed to write manifest");
                // std::fs::write(dest, contents).expect("Failed to write output file");

                // forward the modified object files to the real linker
                let err_file = std::fs::File::options()
                    .append(true)
                    .create(true)
                    .open("/Users/evanalmloff/Desktop/Github/dioxus-test/linker_err.txt")
                    .unwrap();
                let toolchain = if args.contains(&"wasm".to_string()) {
                    "stable-wasm32-unknown-unknown".to_string()
                } else {
                    std::env::var("RUSTUP_TOOLCHAIN").unwrap()
                };

                let mut linker_command = find_linker(toolchain);
                let status = linker_command
                    .args(args.into_iter().skip(1))
                    .stderr(err_file)
                    .status()
                    .expect("Failed to spawn linker");

                if let Some(code) = status.code() {
                    std::process::exit(code);
                }
            }
        }
    }
}

struct AssetReference {
    file: PathBuf,
    byte_span: Range<usize>,
    bundled_asset: BundledAsset,
}

impl AssetReference {
    fn write(&self, new_data: &[u8]) -> std::io::Result<()> {
        let mut binary_data = fs::File::options()
            .write(true)
            .read(true)
            .open(&self.file)?;
        binary_data.seek(std::io::SeekFrom::Start(self.byte_span.start as u64))?;
        // Write the modified binary data back to the file
        binary_data.write_all(&new_data)?;
        binary_data.sync_all()
    }
}

#[derive(Default)]
struct AssetReferences {
    assets: Vec<AssetReference>,
}

impl AssetReferences {
    fn new() -> Self {
        Self { assets: Vec::new() }
    }

    fn add_from_object_path(&mut self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let mut binary_data = fs::File::options().read(true).open(path)?;
        let mut range = None;
        {
            let read_cache = ReadCache::new(&mut binary_data);
            let file = object::File::parse(&read_cache)?;
            for section in file.sections() {
                if section.name()? == "manganis" && section.segment_name()? == Some("__DATA") {
                    if file.format() == object::BinaryFormat::Wasm {
                        // In wasm this is actually the start and end
                        let (start, end) = section.file_range().unwrap();
                        range = Some(start as usize..end as usize);
                    } else {
                        let (offset, len) = section.file_range().unwrap();
                        range = Some(offset as usize..(offset + len) as usize);
                    }
                    break;
                }
            }
        }
        if let Some(range) = range {
            binary_data.seek(std::io::SeekFrom::Start(range.start as u64))?;
            let mut data_in_range = vec![0; range.len()];
            binary_data.read_exact(&mut data_in_range)?;
            for offset in (0..data_in_range.len()).step_by(std::mem::size_of::<BundledAsset>()) {
                let range = (range.start + offset)
                    ..(range.start + offset + std::mem::size_of::<BundledAsset>());
                let const_vec = ConstVec::new().extend(&data_in_range[range.clone()]);
                if let Some((_, bundled_asset)) = deserialize_const!(BundledAsset, const_vec.read())
                {
                    self.assets.push(AssetReference {
                        file: path.clone(),
                        byte_span: range,
                        bundled_asset,
                    });
                }
            }
        }
        Ok(())
    }
}

// find the current linker
fn find_linker(toolchain: String) -> Command {
    let target = toolchain.split("-").nth(1).unwrap();
    match target {
        // usually just ld64 - uses your `cc`
        "aarch64" => {
            // env -u IPHONEOS_DEPLOYMENT_TARGET -u TVOS_DEPLOYMENT_TARGET -u XROS_DEPLOYMENT_TARGET LC_ALL="C" "cc"
            let mut command = Command::new(PathBuf::from("cc"));
            command.env_remove("IPHONEOS_DEPLOYMENT_TARGET");
            command.env_remove("TVOS_DEPLOYMENT_TARGET");
            command.env_remove("XROS_DEPLOYMENT_TARGET");
            command
        }
        "wasm32" => {
            let mut command = Command::new(wasm_ld());
            command.env("LC_ALL", "C");
            command
        }
        _ => todo!("Unsupported target: {}", target),
    }
}

fn wasm_ld() -> PathBuf {
    // eg. /Users/jonkelley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/rust-lld
    //     |_________________________sysroot_____________________________|
    //
    // we should opt to use rust-lld since that's the default on linux and will eventually be the default on windows
    // I think mac will keep ld
    let root = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .unwrap();
    let root = PathBuf::from(String::from_utf8(root.stdout).unwrap().trim())
        .join("lib")
        .join("rustlib")
        .join(Triple::host().to_string())
        .join("bin")
        .join("rust-lld");
    root
}

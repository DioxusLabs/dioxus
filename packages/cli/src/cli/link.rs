use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use target_lexicon::Triple;

/// `dx` can act as a linker in a few scenarios. Note that we don't *actually* implement the linker logic,
/// instead just proxying to a specified linker (or not linking at all!).
///
/// This comes in two flavors:
/// --------------------------
/// - `BaseLink`: We are linking dependencies and want to dynamically select the linker from the environment.
///               This is mostly implemented for Android where the linker is selected in part by the
///               device connected over ADB which can not be determined by .cargo/Config.toml.
///               We implemented this because previous setups like cargo mobile required a hard-coded
///               linker path in your project which does not work in team-based setups.
///
/// - `NoLink`: We are not linking at all, and instead deferring our linking to the driving process,
///             usually being `dx` itself. In this case, we are just writing the linker args to a file
///             and then outputting a dummy object file to satisfy the linker. This is generally used
///             by the binary patching engine since we need to actually do "real linker logic" like
///             traversing object files and satisifying missing symbols. That process is *much* easier
///             to do in the driving host procss when we have all the information available. Unfortuantely,
///             rustc doesn't provide a "real" way of granularly stepping through the compile process
///             so this is basically a hack.
///
/// We use "BaseLink" when a linker is specified, and "NoLink" when it is not. Both generate a resulting
/// object file.
#[derive(Debug, Serialize, Deserialize)]
pub struct LinkAction {
    pub linker: Option<PathBuf>,
    pub triple: Triple,
    pub link_args_file: PathBuf,
    pub link_err_file: PathBuf,
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
        let mut args: Vec<_> = std::env::args().collect();

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

        // Write the linker args to a file for the main process to read
        // todo: we might need to encode these as escaped shell words in case newlines are passed
        std::fs::write(self.link_args_file, args.join("\n"))?;

        // If there's a linker specified, we use that. Otherwise, we write a dummy object file to satisfy
        // any post-processing steps that rustc does.
        match self.linker {
            Some(linker) => {
                let res = std::process::Command::new(linker)
                    .args(args.iter().skip(1))
                    .output()
                    .expect("Failed to run android linker");

                if !res.stderr.is_empty() || !res.stdout.is_empty() {
                    _ = std::fs::create_dir_all(self.link_err_file.parent().unwrap());
                    _ = std::fs::write(
                        self.link_err_file,
                        format!(
                            "Linker error: {}\n{}",
                            String::from_utf8_lossy(&res.stdout),
                            String::from_utf8_lossy(&res.stderr)
                        ),
                    )
                    .unwrap();
                }
            }
            None => {
                // Extract the out path - we're going to write a dummy object file to satisfy the linker
                let out = args.iter().position(|arg| arg == "-o").unwrap();
                let out_file: PathBuf = args[out + 1].clone().into();

                // This creates an object file that satisfies rust's use of llvm-objcopy
                //
                // I'd rather we *not* do this and instead generate a truly linked file (and then delete it) but
                // this at least lets us delay linking until the host compiler is ready.
                //
                // This is because our host compiler is a stateful server and not a stateless linker.
                //
                // todo(jon): do we use Triple::host or the target triple? I think I ran into issues
                // using the target triple, hence the use of "host" but it might not even matter?
                let triple = Triple::host();
                let format = match triple.binary_format {
                    target_lexicon::BinaryFormat::Elf => object::BinaryFormat::Elf,
                    target_lexicon::BinaryFormat::Coff => object::BinaryFormat::Coff,
                    target_lexicon::BinaryFormat::Macho => object::BinaryFormat::MachO,
                    target_lexicon::BinaryFormat::Wasm => object::BinaryFormat::Wasm,
                    target_lexicon::BinaryFormat::Xcoff => object::BinaryFormat::Xcoff,
                    target_lexicon::BinaryFormat::Unknown => todo!(),
                    _ => todo!("Binary format not supported"),
                };

                let arch = match triple.architecture {
                    target_lexicon::Architecture::Wasm32 => object::Architecture::Wasm32,
                    target_lexicon::Architecture::Wasm64 => object::Architecture::Wasm64,
                    target_lexicon::Architecture::X86_64 => object::Architecture::X86_64,
                    target_lexicon::Architecture::Arm(_) => object::Architecture::Arm,
                    target_lexicon::Architecture::Aarch64(_) => object::Architecture::Aarch64,
                    target_lexicon::Architecture::LoongArch64 => object::Architecture::LoongArch64,
                    target_lexicon::Architecture::Unknown => object::Architecture::Unknown,
                    _ => todo!("Architecture not supported"),
                };

                let endian = match triple.endianness() {
                    Ok(target_lexicon::Endianness::Little) => object::Endianness::Little,
                    Ok(target_lexicon::Endianness::Big) => object::Endianness::Big,
                    Err(_) => todo!("Endianness not supported"),
                };

                let bytes = object::write::Object::new(format, arch, endian)
                    .write()
                    .unwrap();

                // Write a dummy object file to satisfy rust/linker since it'll run llvm-objcopy
                // ... I wish it *didn't* do that but I can't tell how to disable the linker without
                // using --emit=obj which is not exactly what we want since that will still pull in
                // the dependencies.
                std::fs::create_dir_all(out_file.parent().unwrap())?;
                std::fs::write(out_file, bytes)?;
            }
        }

        Ok(())
    }
}

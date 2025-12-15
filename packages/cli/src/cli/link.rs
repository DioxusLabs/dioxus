use crate::Result;
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, ffi::OsString, path::PathBuf, process::ExitCode};
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
///             traversing object files and satisfying missing symbols. That process is *much* easier
///             to do in the driving host process when we have all the information available. Unfortunately,
///             rustc doesn't provide a "real" way of granularly stepping through the compile process
///             so this is basically a hack.
///
/// We use "BaseLink" when a linker is specified, and "NoLink" when it is not. Both generate a resulting
/// object file.

#[derive(Debug)]
pub struct LinkAction {
    pub linker: Option<PathBuf>,
    pub triple: Triple,
    pub link_args_file: PathBuf,
    pub link_err_file: PathBuf,
}

/// The linker flavor to use. This influences the argument style that gets passed to the linker.
/// We're imitating the rustc linker flavors here.
///
/// <https://doc.rust-lang.org/beta/nightly-rustc/rustc_target/spec/enum.LinkerFlavor.html>
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum LinkerFlavor {
    Gnu,
    Darwin,
    WasmLld,
    Msvc,
    Unsupported, // a catch-all for unsupported linkers, usually the stripped-down unix ones
}

impl LinkAction {
    const DX_LINK_ARG: &str = "DX_LINK";
    const DX_ARGS_FILE: &str = "DX_LINK_ARGS_FILE";
    const DX_ERR_FILE: &str = "DX_LINK_ERR_FILE";
    const DX_LINK_TRIPLE: &str = "DX_LINK_TRIPLE";
    const DX_LINK_CUSTOM_LINKER: &str = "DX_LINK_CUSTOM_LINKER";

    /// Should we write the input arguments to a file (aka act as a linker subprocess)?
    ///
    /// Just check if the magic env var is set
    pub(crate) fn from_env() -> Option<Self> {
        if std::env::var(Self::DX_LINK_ARG).is_err() {
            return None;
        }

        Some(Self {
            linker: std::env::var(Self::DX_LINK_CUSTOM_LINKER)
                .ok()
                .map(PathBuf::from),
            link_args_file: std::env::var(Self::DX_ARGS_FILE)
                .expect("Linker args file not set")
                .into(),
            link_err_file: std::env::var(Self::DX_ERR_FILE)
                .expect("Linker error file not set")
                .into(),
            triple: std::env::var(Self::DX_LINK_TRIPLE)
                .expect("Linker triple not set")
                .parse()
                .expect("Failed to parse linker triple"),
        })
    }

    pub(crate) fn write_env_vars(
        &self,
        env_vars: &mut Vec<(Cow<'static, str>, OsString)>,
    ) -> Result<()> {
        env_vars.push((Self::DX_LINK_ARG.into(), "1".into()));
        env_vars.push((
            Self::DX_ARGS_FILE.into(),
            dunce::canonicalize(&self.link_args_file)?.into_os_string(),
        ));
        env_vars.push((
            Self::DX_ERR_FILE.into(),
            dunce::canonicalize(&self.link_err_file)?.into_os_string(),
        ));
        env_vars.push((Self::DX_LINK_TRIPLE.into(), self.triple.to_string().into()));
        if let Some(linker) = &self.linker {
            env_vars.push((
                Self::DX_LINK_CUSTOM_LINKER.into(),
                dunce::canonicalize(linker)
                    .unwrap_or(linker.clone())
                    .into_os_string(),
            ));
        }

        Ok(())
    }

    pub(crate) fn run_link(self) -> ExitCode {
        let link_err_file = self.link_err_file.clone();
        if let Err(err) = self.run_link_inner() {
            eprintln!("Linker error: {err}");

            // If we failed to run the linker, we need to write the error to the file
            // so that the main process can read it.
            _ = std::fs::create_dir_all(link_err_file.parent().unwrap());
            _ = std::fs::write(link_err_file, format!("Linker error: {err}"));

            return ExitCode::FAILURE;
        }

        ExitCode::SUCCESS
    }

    /// Write the incoming linker args to a file
    ///
    /// The file will be given by the dx-magic-link-arg env var itself, so we use
    /// it both for determining if we should act as a linker and the for the file name itself.
    fn run_link_inner(self) -> Result<()> {
        let args: Vec<_> = std::env::args().collect();
        if args.is_empty() {
            return Ok(());
        }

        let mut args = get_actual_linker_args_excluding_program_name(args);

        if self.triple.environment == target_lexicon::Environment::Android {
            args.retain(|arg| !arg.ends_with(".lib"));
        }

        // Write the linker args to a file for the main process to read
        // todo: we might need to encode these as escaped shell words in case newlines are passed
        std::fs::write(&self.link_args_file, args.join("\n"))?;

        // If there's a linker specified, we use that. Otherwise, we write a dummy object file to satisfy
        // any post-processing steps that rustc does.
        match self.linker {
            Some(linker) => {
                let mut cmd = std::process::Command::new(linker);
                match cfg!(target_os = "windows") {
                    true => cmd.arg(format!("@{}", &self.link_args_file.display())),
                    false => cmd.args(args),
                };
                let res = cmd.output().expect("Failed to run linker");

                if !res.status.success() {
                    bail!(
                        "{}\n{}",
                        String::from_utf8_lossy(&res.stdout),
                        String::from_utf8_lossy(&res.stderr)
                    );
                }
                if !res.stderr.is_empty() || !res.stdout.is_empty() {
                    // Write linker warnings to file so that the main process can read them.
                    _ = std::fs::create_dir_all(self.link_err_file.parent().unwrap());
                    _ = std::fs::write(
                        self.link_err_file,
                        format!(
                            "Linker warnings: {}\n{}",
                            String::from_utf8_lossy(&res.stdout),
                            String::from_utf8_lossy(&res.stderr)
                        ),
                    );
                }
            }
            None => {
                // Extract the out path - we're going to write a dummy object file to satisfy the linker
                let out_file: PathBuf = match self.triple.operating_system {
                    target_lexicon::OperatingSystem::Windows => {
                        let out_arg = args.iter().find(|arg| arg.starts_with("/OUT")).unwrap();
                        out_arg.trim_start_matches("/OUT:").to_string().into()
                    }
                    _ => {
                        let out = args.iter().position(|arg| arg == "-o").unwrap();
                        args[out + 1].clone().into()
                    }
                };

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
                    target_lexicon::BinaryFormat::Unknown => unimplemented!(),
                    _ => unimplemented!("Binary format not supported"),
                };

                let arch = match triple.architecture {
                    target_lexicon::Architecture::Wasm32 => object::Architecture::Wasm32,
                    target_lexicon::Architecture::Wasm64 => object::Architecture::Wasm64,
                    target_lexicon::Architecture::X86_64 => object::Architecture::X86_64,
                    target_lexicon::Architecture::Arm(_) => object::Architecture::Arm,
                    target_lexicon::Architecture::Aarch64(_) => object::Architecture::Aarch64,
                    target_lexicon::Architecture::LoongArch64 => object::Architecture::LoongArch64,
                    target_lexicon::Architecture::Unknown => object::Architecture::Unknown,
                    _ => unimplemented!("Architecture not supported"),
                };

                let endian = match triple.endianness() {
                    Ok(target_lexicon::Endianness::Little) => object::Endianness::Little,
                    Ok(target_lexicon::Endianness::Big) => object::Endianness::Big,
                    Err(_) => unimplemented!("Endianness not supported"),
                };

                let bytes = object::write::Object::new(format, arch, endian)
                    .write()
                    .context("Failed to emit stub link file")?;

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

pub fn get_actual_linker_args_excluding_program_name(args: Vec<String>) -> Vec<String> {
    args.into_iter()
        .skip(1) // the first arg is program name
        .flat_map(|arg| handle_linker_arg_response_file(arg).into_iter())
        .collect()
}

// handle Windows linker response file. It's designed to workaround Windows command length limit.
// https://learn.microsoft.com/en-us/cpp/build/reference/at-specify-a-linker-response-file?view=msvc-170
pub fn handle_linker_arg_response_file(arg: String) -> Vec<String> {
    if arg.starts_with('@') {
        let path = arg.trim().trim_start_matches('@');
        let file_binary = std::fs::read(path).unwrap();

        // This may be a utf-16le file. Let's try utf-8 first.
        let mut content = String::from_utf8(file_binary.clone()).unwrap_or_else(|_| {
            // Convert Vec<u8> to Vec<u16> to convert into a String
            let binary_u16le: Vec<u16> = file_binary
                .chunks_exact(2)
                .map(|a| u16::from_le_bytes([a[0], a[1]]))
                .collect();

            String::from_utf16_lossy(&binary_u16le)
        });

        // Remove byte order mark in the beginning
        if content.starts_with('\u{FEFF}') {
            content.remove(0);
        }

        // Gather linker args, and reset the args to be just the linker args
        content
            .lines()
            .map(|line| {
                let line_parsed = line.trim().to_string();
                let line_parsed = line_parsed.trim_end_matches('"').to_string();
                let line_parsed = line_parsed.trim_start_matches('"').to_string();
                line_parsed
            })
            .collect()
    } else {
        vec![arg]
    }
}

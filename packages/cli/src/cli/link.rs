use crate::{Platform, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use target_lexicon::Triple;
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
        save_link_args: PathBuf,
        triple: Triple,
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
                let mut args = args
                    .into_iter()
                    .skip(1)
                    .filter(|arg| arg != "-Wl,-dead_strip" && !strip)
                    .collect::<Vec<String>>();

                // this is ld64 only, we need --whole-archive for gnu/ld
                args.push("-Wl,-all_load".to_string());

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
                save_link_args,
                triple,
            } => {
                // Write the linker args to a file for the main process to read
                std::fs::write(save_link_args, args.join("\n"))?;

                // Extract the out
                let out = args.iter().position(|arg| arg == "-o").unwrap();
                let out_file: PathBuf = args[out + 1].clone().into();

                // Write a dummy object file to satisfy rust/linker since it'll run llvm-objcopy
                // ... I wish it *didn't* do that but I can't tell how to disable the linker without
                // using --emit=obj which is not exactly what we want since that will still pull in
                // the dependencies.
                std::fs::create_dir_all(out_file.parent().unwrap())?;
                std::fs::write(out_file, make_dummy_object_file(triple))?;
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

fn make_dummy_object_file(triple: Triple) -> Vec<u8> {
    let format = match triple.binary_format {
        target_lexicon::BinaryFormat::Unknown => todo!(),
        target_lexicon::BinaryFormat::Elf => object::BinaryFormat::Elf,
        target_lexicon::BinaryFormat::Coff => object::BinaryFormat::Coff,
        target_lexicon::BinaryFormat::Macho => object::BinaryFormat::MachO,
        target_lexicon::BinaryFormat::Wasm => object::BinaryFormat::Wasm,
        target_lexicon::BinaryFormat::Xcoff => object::BinaryFormat::Xcoff,
        _ => todo!(),
    };

    let arch = match triple.architecture {
        target_lexicon::Architecture::Wasm32 => object::Architecture::Wasm32,
        target_lexicon::Architecture::Wasm64 => object::Architecture::Wasm64,
        target_lexicon::Architecture::X86_64 => object::Architecture::X86_64,
        target_lexicon::Architecture::Arm(arm_architecture) => object::Architecture::Arm,
        target_lexicon::Architecture::Aarch64(aarch64_architecture) => {
            object::Architecture::Aarch64
        }
        target_lexicon::Architecture::LoongArch64 => object::Architecture::LoongArch64,
        target_lexicon::Architecture::Unknown => object::Architecture::Unknown,
        _ => todo!(),
    };

    let endian = match triple.endianness() {
        Ok(target_lexicon::Endianness::Little) => object::Endianness::Little,
        Ok(target_lexicon::Endianness::Big) => object::Endianness::Big,
        Err(_) => todo!(),
    };

    // todo: these are architecture specific but should not be
    let dummy_object_file = object::write::Object::new(format, arch, endian);

    dummy_object_file.write().unwrap()
}

#[test]
fn creates_dummy_object_file_ios() {
    let dummy_object_file = make_dummy_object_file(Platform::Ios);
}

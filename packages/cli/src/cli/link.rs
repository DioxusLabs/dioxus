use crate::Result;
use anyhow::Context;
use anyhow::Context;
use const_serialize::ConstVec;
use dioxus_cli_opt::{process_file_to, AssetManifest};
use manganis::BundledAsset;
use object::{Object, ObjectSection, ReadCache};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Debug;
use std::fs::{self, create_dir_all};
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::process::Command;
use target_lexicon::Triple;
use target_lexicon::Triple;
use tempfile::NamedTempFile;

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
/// https://doc.rust-lang.org/beta/nightly-rustc/rustc_target/spec/enum.LinkerFlavor.html
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum LinkerFlavor {
    Gnu,
    Darwin,
    WasmLld,
    Unix,
    Msvc,
}

impl LinkAction {
    const DX_LINK_ARG: &str = "DX_LINK";
    const DX_ARGS_FILE: &str = "DX_LINK_ARGS_FILE";
    const DX_ERR_FILE: &str = "DX_LINK_ERR_FILE";
    const DX_LINK_TRIPLE: &str = "DX_LINK_TRIPLE";
    const DX_LINK_CUSTOM_LINKER: &str = "DX_LINK_CUSTOM_LINKER";

    // Publicly documented CLI APIs for linking
    pub(crate) const ENV_VAR_NAME_ASSETS_TARGET: &'static str = "DX_LINK_ASSETS_TARGET"; // The target directory for the assets
    pub(crate) const LOG_FILE_VAR_NAME: &'static str = "DX_LINK_LOG_FILE"; // The log file to use

    /// Should we write the input arguments to a file (aka act as a linker subprocess)?
    ///
    /// Just check if the magic env var is set
    pub(crate) fn from_env() -> Option<Self> {
        if let Ok(target) = std::env::var(Self::ENV_VAR_NAME_ASSETS_TARGET) {
            return Some(LinkAction::OptimizeAssets {
                destination: PathBuf::from(target),
            });
        }

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

    pub(crate) fn write_env_vars(&self, env_vars: &mut Vec<(&str, String)>) -> Result<()> {
        env_vars.push((Self::DX_LINK_ARG, "1".to_string()));
        env_vars.push((
            Self::DX_ARGS_FILE,
            dunce::canonicalize(&self.link_args_file)?
                .to_string_lossy()
                .to_string(),
        ));
        env_vars.push((
            Self::DX_ERR_FILE,
            dunce::canonicalize(&self.link_err_file)?
                .to_string_lossy()
                .to_string(),
        ));
        env_vars.push((Self::DX_LINK_TRIPLE, self.triple.to_string()));
        if let Some(linker) = &self.linker {
            env_vars.push((
                Self::DX_LINK_CUSTOM_LINKER,
                dunce::canonicalize(linker)?.to_string_lossy().to_string(),
            ));
        }

        Ok(())
    }

    pub(crate) async fn run_link(self) {
        let link_err_file = self.link_err_file.clone();
        let res = self.run_link_inner().await;

        if let Err(err) = res {
            // If we failed to run the linker, we need to write the error to the file
            // so that the main process can read it.
            _ = std::fs::create_dir_all(link_err_file.parent().unwrap());
            _ = std::fs::write(link_err_file, format!("Linker error: {err}"));
        }
    }

    /// Write the incoming linker args to a file
    ///
    /// The file will be given by the dx-magic-link-arg env var itself, so we use
    /// it both for determining if we should act as a linker and the for the file name itself.
    async fn run_link_inner(self) -> Result<()> {
        init_linker_logger();

        let mut args: Vec<_> = std::env::args().collect();
        if args.is_empty() {
            return Ok(());
        }

        handle_linker_command_file(&mut args);

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
                    .expect("Failed to run linker");

                if !res.stderr.is_empty() || !res.stdout.is_empty() {
                    _ = std::fs::create_dir_all(self.link_err_file.parent().unwrap());
                    _ = std::fs::write(
                        self.link_err_file,
                        format!(
                            "Linker error: {}\n{}",
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

    fn link_asset_manifest() -> (AssetManifest, std::process::ExitStatus) {
        let args: Vec<_> = std::env::args().collect();
        let mut references = AssetReferences::from_link_args(&args);

        // Hash each file in parallel
        references.assets.par_iter_mut().for_each(|asset| {
            dioxus_cli_opt::add_hash_to_asset(&mut asset.bundled_asset);
        });

        // Look for --flavor wasm in the args
        let targeting_wasm =
            args.contains(&"-flavor".to_string()) && args.contains(&"wasm".to_string());
        let mut linker_args = args.into_iter().skip(1).collect::<Vec<_>>();
        let mut _tempfile_handle = None;

        // If we are targeting wasm, create an object file to satisfy the imports
        if targeting_wasm {
            let mut data_sections = Vec::new();
            for asset in references.assets.iter() {
                let name = asset.bundled_asset.link_section();
                let data = const_serialize::serialize_const(&asset.bundled_asset, ConstVec::new());
                data_sections.push((name, data.as_ref().to_vec()));
            }

            // Create the object file
            let object_file = create_data_object_file(
                data_sections
                    .iter()
                    .map(|(name, data)| (*name, data.as_ref())),
            );
            let mut temp_file = NamedTempFile::new().expect("Failed to create temporary file");
            temp_file
                .write_all(&object_file)
                .expect("Failed to write object file");
            linker_args.push(temp_file.path().to_string_lossy().to_string());
            _tempfile_handle = Some(temp_file);
        }
        // Otherwise overwrite the object files
        else {
            for asset in &references.assets {
                // Write the asset to the object file
                if let Err(err) = asset.write() {
                    tracing::error!("Failed to write asset to object file: {err}");
                }
            }

            Ok(())
        }

        // // Assemble an asset manifest by walking the object files being passed to us
        // LinkAction::BuildAssetManifest { destination: dest } => {
        //     let (manifest, status) = link_asset_manifest();

        //     let contents =
        //         serde_json::to_string(&manifest).context("Failed to write manifest")?;
        //     std::fs::write(dest, contents).context("Failed to write output file")?;

        //     if let Some(code) = status.code() {
        //         std::process::exit(code);
        //     }
        // }

        // // Optimize the assets by copying them to the destination
        // LinkAction::OptimizeAssets { destination } => {
        //     let (manifest, status) = link_asset_manifest();
        //     if let Err(err) = create_dir_all(&destination) {
        //         tracing::error!("Failed to create destination directory: {err}");
        //     }
        //     for asset in manifest.assets() {
        //         let path = PathBuf::from(asset.absolute_source_path());
        //         let destination_path = destination.join(asset.bundled_path());
        //         tracing::debug!(
        //             "Processing asset {} --> {} {:#?}",
        //             path.display(),
        //             destination_path.display(),
        //             asset
        //         );
        //         process_file_to(asset.options(), &path, &destination_path)?;
        //     }
        //     if let Some(code) = status.code() {
        //         std::process::exit(code);
        //     }
        // }
    }
}

pub fn handle_linker_command_file(args: &mut Vec<String>) {
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
        *args = content
            .lines()
            .map(|line| {
                let line_parsed = line.trim().to_string();
                let line_parsed = line_parsed.trim_end_matches('"').to_string();
                let line_parsed = line_parsed.trim_start_matches('"').to_string();
                line_parsed
            })
            .collect();
    }

    // Extract the manifest from the hashed assets
    let mut manifest = AssetManifest::default();
    for asset in references.assets.iter() {
        // Add the asset to the manifest
        manifest.insert_asset(asset.bundled_asset);
    }

    // forward the modified object files to the real linker
    let toolchain = if targeting_wasm {
        "stable-wasm32-unknown-unknown".to_string()
    } else {
        std::env::var("RUSTUP_TOOLCHAIN").unwrap()
    };

    let mut linker_command = find_linker(toolchain);
    let status = linker_command
        .args(linker_args)
        .status()
        .expect("Failed to spawn linker");

    tracing::info!("Found assets: {:#?}", manifest.assets().collect::<Vec<_>>());

    (manifest, status)
}

fn linker_log_file() -> Option<PathBuf> {
    std::env::var(LinkAction::LOG_FILE_VAR_NAME)
        .ok()
        .map(PathBuf::from)
}

fn init_linker_logger() {
    if let Some(log_path) = linker_log_file() {
        let log_file = std::fs::File::options()
            .append(true)
            .create(true)
            .open(log_path)?;
        tracing_subscriber::fmt()
            .with_writer(log_file)
            .with_max_level(tracing::Level::DEBUG)
            .compact()
            .with_ansi(false)
            .init();
    }
}

struct AssetReference {
    file: PathBuf,
    offset: usize,
    bundled_asset: BundledAsset,
}

impl AssetReference {
    fn write(&self) -> std::io::Result<()> {
        let new_data = ConstVec::new();
        let new_data = const_serialize::serialize_const(&self.bundled_asset, new_data);

        let mut binary_data = fs::File::options()
            .write(true)
            .read(true)
            .open(&self.file)?;
        binary_data.seek(std::io::SeekFrom::Start(self.offset as u64))?;
        // Write the modified binary data back to the file
        binary_data.write_all(new_data.as_ref())?;
        binary_data.sync_all()?;

        Ok(())
    }
}

fn collect_object_files_from_args(args: &[String]) -> Vec<PathBuf> {
    let mut obj_args = args.to_vec();
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

    obj_args.iter().map(PathBuf::from).collect()
}

#[derive(Default)]
struct AssetReferences {
    assets: Vec<AssetReference>,
}

impl AssetReferences {
    fn from_link_args(args: &[String]) -> Self {
        let mut references = AssetReferences::default();
        let obj_files = collect_object_files_from_args(args);
        for file in obj_files {
            if let Ok(path) = file.canonicalize() {
                if let Err(err) = references.add_from_object_path(&path) {
                    tracing::error!("Failed to read object file {}: {err}", path.display());
                }
            }
        }
        references
    }

    fn add_from_object_path(&mut self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let mut binary_data = fs::File::options().read(true).open(path)?;
        let mut range = None;
        {
            let read_cache = ReadCache::new(&mut binary_data);
            let file = object::File::parse(&read_cache)?;
            for section in file.sections() {
                if let Ok(name) = section.name() {
                    if manganis_core::linker::LinkSection::ALL
                        .iter()
                        .any(|link_section| link_section.link_section == name)
                    {
                        let Some(file_range) = section.file_range() else {
                            continue;
                        };
                        if file.format() == object::BinaryFormat::Wasm {
                            // In wasm this is actually the start and end
                            let (start, end) = file_range;
                            range = Some(start as usize..end as usize);
                        } else {
                            let (offset, len) = file_range;
                            range = Some(offset as usize..(offset + len) as usize);
                        }
                        break;
                    }
                }
            }
        }
        if let Some(range) = range {
            binary_data.seek(std::io::SeekFrom::Start(range.start as u64))?;
            let mut data_in_range = vec![0; range.len()];
            binary_data.read_exact(&mut data_in_range)?;
            let mut offset = 0;
            let mut buffer = const_serialize::ConstReadBuffer::new(&data_in_range);

            while let Some((remaining_buffer, bundled_asset)) =
                const_serialize::deserialize_const!(BundledAsset, buffer)
            {
                let len = (data_in_range.len() - remaining_buffer.remaining().len()) - offset;
                self.assets.push(AssetReference {
                    file: path.clone(),
                    offset: range.start + offset,
                    bundled_asset,
                });
                offset += len;
                buffer = remaining_buffer;
            }
        }
        Ok(())
    }
}

// find the current linker
fn find_linker(toolchain: String) -> Command {
    // If there is a linker environment variable, use that
    if let Ok(linker) = std::env::var(LinkAction::DX_LINKER_ENV_VAR) {
        return Command::new(linker);
    }

    let target = toolchain.split("-").collect::<Vec<_>>();
    tracing::info!("Linking for target: {:?}", target);
    match target.as_slice() {
        // usually just ld64 - uses your `cc`
        // Eg. aarch64-apple-darwin
        [_, _, "apple", _] => {
            let mut command = Command::new(PathBuf::from("cc"));
            command.env_remove("IPHONEOS_DEPLOYMENT_TARGET");
            command.env_remove("TVOS_DEPLOYMENT_TARGET");
            command.env_remove("XROS_DEPLOYMENT_TARGET");
            command
        }
        // Eg. nightly-x86_64-unknown-linux-gnu
        [_, arch, _, "linux", _] => {
            let mut command = Command::new("cc");
            command.env("LC_ALL", "C");
            if arch.contains("64") {
                command.arg("-m64");
            }
            command
        }
        // Eg. stable-x86_64-pc-windows-msvc
        [_, _, _, "windows", _] => {
            let mut command = Command::new("link.exe");
            command.arg("/NOLOGO");
            command
        }
        // Eg. nightly-wasm32-unknown-unknown
        [_, "wasm32", _, _] => {
            let mut command = Command::new(wasm_ld());
            command.env("LC_ALL", "C");
            command
        }
        _ => {
            panic!(
                "Unknown target {}. Please set the environment variable DIOXUS_LINKER to the path of your linker.
If you don't know where your linker is, create a blank rust file and run `rustc temp.rs --print link-args`.
On unix-like platforms, you can run this command to find your link args:
`echo \"fn main(){{}}\" > ./temp.rs && rustc temp.rs --print link-args -Z unstable-options && rm ./temp.rs`
Once you find the linker args for your platform feel free to open an issue with link args so we can
add support for the platform out of the box: https://github.com/DioxusLabs/dioxus/issues/new", target.join("-")
        )
        }
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

fn create_data_object_file<'a>(
    data_sections: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Vec<u8> {
    use wasm_encoder::{
        ConstExpr, DataSection, DataSymbolDefinition, LinkingSection, Module, SymbolTable,
    };

    let mut linking = LinkingSection::new();
    let mut sym_tab = SymbolTable::new();
    let mut data_section = DataSection::new();

    let memory_index = 0;
    let mut offset = 0;
    let mut all_bytes = Vec::new();

    for (symbol_name, data) in data_sections {
        let flags = SymbolTable::WASM_SYM_EXPORTED;
        let size = data.len() as u32;

        all_bytes.extend_from_slice(data);
        sym_tab.data(
            flags,
            symbol_name,
            Some(DataSymbolDefinition {
                index: memory_index,
                offset: offset as u32,
                size,
            }),
        );
        linking.symbol_table(&sym_tab);
        offset += data.len();
    }
    data_section.active(memory_index, &ConstExpr::i32_const(0), all_bytes);

    // Add the linking section to a new Wasm module and get the encoded bytes.
    let mut module = Module::new();
    module.section(&data_section).section(&linking);
    module.finish()
}

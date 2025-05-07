use crate::Result;
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
    pub link_args_file: Option<PathBuf>,
    pub link_err_file: Option<PathBuf>,
    pub link_log_file: Option<PathBuf>,
    pub link_asset_manifest_file: Option<PathBuf>,
    pub link_asset_out_dir: Option<PathBuf>,
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
    const DX_LINK_ASSET_MANIFEST: &str = "DX_LINK_ASSET_MANIFEST";

    // Publicly documented CLI APIs for linking
    pub(crate) const ENV_VAR_NAME_ASSETS_TARGET: &'static str = "DX_LINK_ASSETS_TARGET"; // The target directory for the assets
    pub(crate) const LOG_FILE_VAR_NAME: &'static str = "DX_LINK_LOG_FILE"; // The log file to use

    /// Should we write the input arguments to a file (aka act as a linker subprocess)?
    ///
    /// Just check if the magic env var is set
    pub(crate) fn from_env() -> Option<Self> {
        if std::env::var(Self::DX_LINK_ARG).is_err()
            && std::env::var(Self::ENV_VAR_NAME_ASSETS_TARGET).is_err()
        {
            return None;
        }

        Some(Self {
            linker: std::env::var(Self::DX_LINK_CUSTOM_LINKER)
                .ok()
                .map(PathBuf::from),
            link_args_file: std::env::var(Self::DX_ARGS_FILE).ok().map(PathBuf::from),
            link_err_file: std::env::var(Self::DX_ERR_FILE).ok().map(PathBuf::from),
            triple: std::env::var(Self::DX_LINK_TRIPLE)
                .expect("Linker triple not set")
                .parse()
                .expect("Failed to parse linker triple"),
            link_asset_manifest_file: std::env::var(Self::DX_LINK_ASSET_MANIFEST)
                .ok()
                .map(PathBuf::from),
            link_asset_out_dir: std::env::var(Self::ENV_VAR_NAME_ASSETS_TARGET)
                .ok()
                .map(PathBuf::from),
            link_log_file: std::env::var(Self::LOG_FILE_VAR_NAME)
                .ok()
                .map(PathBuf::from),
        })
    }

    pub(crate) fn write_env_vars(&self, env_vars: &mut Vec<(&str, String)>) -> Result<()> {
        env_vars.push((Self::DX_LINK_ARG, "1".to_string()));
        if let Some(link_args_file) = &self.link_args_file {
            env_vars.push((
                Self::DX_ARGS_FILE,
                dunce::canonicalize(&link_args_file)?
                    .to_string_lossy()
                    .to_string(),
            ));
        }
        if let Some(link_err_file) = &self.link_err_file {
            env_vars.push((
                Self::DX_ERR_FILE,
                dunce::canonicalize(&link_err_file)?
                    .to_string_lossy()
                    .to_string(),
            ));
        }
        env_vars.push((Self::DX_LINK_TRIPLE, self.triple.to_string()));
        if let Some(linker) = &self.linker {
            env_vars.push((
                Self::DX_LINK_CUSTOM_LINKER,
                dunce::canonicalize(linker)?.to_string_lossy().to_string(),
            ));
        }
        if let Some(link_asset_manifest_file) = &self.link_asset_manifest_file {
            env_vars.push((
                Self::DX_LINK_ASSET_MANIFEST,
                dunce::canonicalize(link_asset_manifest_file)?
                    .to_string_lossy()
                    .to_string(),
            ));
        }
        if let Some(link_asset_out_dir) = &self.link_asset_out_dir {
            env_vars.push((
                Self::ENV_VAR_NAME_ASSETS_TARGET,
                dunce::canonicalize(link_asset_out_dir)?
                    .to_string_lossy()
                    .to_string(),
            ));
        }
        if let Some(link_log_file) = &self.link_log_file {
            env_vars.push((
                Self::LOG_FILE_VAR_NAME,
                dunce::canonicalize(link_log_file)?
                    .to_string_lossy()
                    .to_string(),
            ));
        }

        Ok(())
    }

    pub(crate) async fn run_link(self) {
        let link_err_file = self.link_err_file.clone();
        let res = self.run_link_inner().await;

        if let Err(err) = res {
            match &link_err_file {
                Some(link_err_file) => {
                    // If we failed to run the linker, we need to write the error to the file
                    // so that the main process can read it.
                    _ = std::fs::create_dir_all(link_err_file.parent().unwrap());
                    _ = std::fs::write(link_err_file, format!("Linker error: {err}"));
                }
                None => {
                    tracing::error!("Failed to run linker: {err}");
                }
            }
        }
    }

    /// Write the incoming linker args to a file
    ///
    /// The file will be given by the dx-magic-link-arg env var itself, so we use
    /// it both for determining if we should act as a linker and the for the file name itself.
    async fn run_link_inner(self) -> Result<()> {
        self.init_linker_logger()?;

        let mut args: Vec<_> = std::env::args().collect();
        if args.is_empty() {
            return Ok(());
        }

        handle_linker_command_file(&mut args);

        if let Some(link_args_file) = &self.link_args_file {
            // Write the linker args to a file for the main process to read
            // todo: we might need to encode these as escaped shell words in case newlines are passed
            std::fs::write(link_args_file, args.join("\n"))?;
        }

        // If there's a linker specified, we use that. Otherwise, we write a dummy object file to satisfy
        // any post-processing steps that rustc does.
        match &self.linker {
            Some(linker) => {
                let res = std::process::Command::new(linker)
                    .args(args.iter().skip(1))
                    .output()
                    .expect("Failed to run linker");

                if !res.stderr.is_empty() || !res.stdout.is_empty() {
                    let message = format!(
                        "Linker error: {}\n{}",
                        String::from_utf8_lossy(&res.stdout),
                        String::from_utf8_lossy(&res.stderr)
                    );
                    if let Some(link_err_file) = &self.link_err_file {
                        _ = std::fs::create_dir_all(link_err_file.parent().unwrap());
                        _ = std::fs::write(link_err_file, message);
                    } else {
                        tracing::error!("Failed to run linker: {message}");
                    }
                }
            }
            None => {
                // Extract the out path - we're going to write a dummy object file to satisfy the linker
                let out_file: PathBuf = self.out_path(&args);

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

    fn out_path(&self, args: &[String]) -> PathBuf {
        match self.triple.operating_system {
            target_lexicon::OperatingSystem::Windows => {
                let out_arg = args.iter().find(|arg| arg.starts_with("/OUT")).unwrap();
                out_arg.trim_start_matches("/OUT:").to_string().into()
            }
            _ => {
                let out = args.iter().position(|arg| arg == "-o").unwrap();
                args[out + 1].clone().into()
            }
        }
    }

    fn link_asset_manifest(&self, args: &mut Vec<String>) -> Result<()> {
        let mut references = AssetReferences::from_link_args(&args);

        // Hash each file in parallel
        references.assets.par_iter_mut().for_each(|asset| {
            dioxus_cli_opt::add_hash_to_asset(&mut asset.bundled_asset);
        });

        // Look for --flavor wasm in the args
        let targeting_wasm = self.triple.architecture == target_lexicon::Architecture::Wasm32
            || self.triple.architecture == target_lexicon::Architecture::Wasm64;

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
            let asset_file = self.out_path(&args).join("manganis_assets_out");
            std::fs::write(asset_file.with_extension("o"), object_file)
                .context("Failed to write object file")?;
            args.push(asset_file.to_string_lossy().to_string());
        }
        // Otherwise overwrite the object files
        else {
            for asset in &references.assets {
                // Write the asset to the object file
                if let Err(err) = asset.write() {
                    tracing::error!("Failed to write asset to object file: {err}");
                }
            }
        }

        // Extract the manifest from the hashed assets
        let mut manifest = AssetManifest::default();
        for asset in references.assets.iter() {
            // Add the asset to the manifest
            manifest.insert_asset(asset.bundled_asset);
        }

        tracing::info!("Found assets: {:#?}", manifest.assets().collect::<Vec<_>>());

        if let Some(link_asset_manifest_file) = &self.link_asset_manifest_file {
            // Write the asset manifest to the file
            let contents =
                serde_json::to_string(&manifest).context("Failed to write asset manifest")?;
            std::fs::write(link_asset_manifest_file, contents)
                .context("Failed to write asset manifest file")?;
        }

        if let Some(link_asset_out_dir) = &self.link_asset_out_dir {
            if let Err(err) = create_dir_all(&link_asset_out_dir) {
                tracing::error!("Failed to create destination directory: {err}");
            }
            for asset in manifest.assets() {
                let path = PathBuf::from(asset.absolute_source_path());
                let destination_path = link_asset_out_dir.join(asset.bundled_path());
                tracing::debug!(
                    "Processing asset {} --> {} {:#?}",
                    path.display(),
                    destination_path.display(),
                    asset
                );
                process_file_to(asset.options(), &path, &destination_path)?;
            }
        }

        Ok(())
    }

    fn init_linker_logger(&self) -> Result<()> {
        if let Some(log_path) = &self.link_log_file {
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
        Ok(())
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

#[derive(Default)]
struct AssetReferences {
    assets: Vec<AssetReference>,
}

impl AssetReferences {
    fn from_link_args(args: &[String]) -> Self {
        let mut args = args.to_vec();
        let mut references = AssetReferences::default();
        handle_linker_command_file(&mut args);
        for file in args {
            let path = PathBuf::from(file);
            if path.extension().map_or(false, |ext| ext == "o") {
                if let Ok(path) = path.canonicalize() {
                    if let Err(err) = references.add_from_object_path(&path) {
                        tracing::error!("Failed to read object file {}: {err}", path.display());
                    }
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

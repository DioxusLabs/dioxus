use const_serialize::ConstVec;
use dioxus_cli_opt::AssetManifest;
use manganis::BundledAsset;
use object::{Object, ObjectSection, ReadCache};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Debug;
use std::fs;
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::process::Command;
use target_lexicon::Triple;
use tempfile::NamedTempFile;

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
        if let Some(log_path) = linker_log_file() {
            let log_file = std::fs::File::options()
                .append(true)
                .create(true)
                .open(log_path)
                .unwrap();
            tracing_subscriber::fmt()
                .with_writer(log_file)
                .with_max_level(tracing::Level::DEBUG)
                .compact()
                .with_ansi(false)
                .init();
        }

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
                let mut references = AssetReferences::default();

                let mut obj_args = args.clone();
                references.assets.clear();
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
                        _ = references.add_from_object_path(&path);
                    }
                }

                // Hash each file in parallel
                references.assets.par_iter_mut().for_each(|asset| {
                    dioxus_cli_opt::add_hash_to_asset(&mut asset.bundled_asset);
                });

                let targeting_wasm = args.contains(&"wasm".to_string());
                let mut linker_args = args.into_iter().skip(1).collect::<Vec<_>>();
                let mut _tempfile_handle = None;

                // If we are targeting wasm, create an object file to satisfy the imports
                if targeting_wasm {
                    let mut data_sections = Vec::new();
                    for asset in references.assets.iter() {
                        let name = asset.bundled_asset.link_section();
                        let data =
                            const_serialize::serialize_const(&asset.bundled_asset, ConstVec::new());
                        const_serialize::deserialize_const!(BundledAsset, data.read()).unwrap();
                        data_sections.push((name, data.as_ref().to_vec()));
                    }

                    // Create the object file
                    let object_file = create_data_object_file(
                        data_sections
                            .iter()
                            .map(|(name, data)| (*name, data.as_ref())),
                    );
                    let mut temp_file =
                        NamedTempFile::new().expect("Failed to create temporary file");
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
                }

                // Extract the manifest from the hashed assets
                let mut manifest = AssetManifest::default();
                for asset in references.assets.iter() {
                    // Add the asset to the manifest
                    manifest.insert_asset(asset.bundled_asset);
                }

                let contents = serde_json::to_string(&manifest).expect("Failed to write manifest");
                std::fs::write(dest, contents).expect("Failed to write output file");

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

                if let Some(code) = status.code() {
                    std::process::exit(code);
                }
            }
        }
    }
}

fn linker_log_file() -> Option<PathBuf> {
    std::env::var("DIOXUS_LINKER_LOG_FILE")
        .ok()
        .map(PathBuf::from)
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
    if let Ok(linker) = std::env::var("DIOXUS_LINKER") {
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
        [_, _, _, "linux", _] => {
            let mut command = Command::new("cc");
            command.env("LC_ALL", "C");
            command
        }
        // Eg. x86_64-pc-windows-msvc
        [_, _, "windows", _] => {
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

// fn linux_ld() -> PathBuf {
//     LC_ALL="C" PATH="/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin:/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/self-contained:/home/evan/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/games:/usr/local/games:/snap/bin:/snap/bin" VSLANG="1033" "cc" "-m64" "/tmp/rustcI2egGS/symbols.o" "main.main.4cac11b5fb976cef-cgu.0.rcgu.o" "main.97rcuhy2qxy2iu2fheg5t5ywl.rcgu.o" "-Wl,--as-needed" "-Wl,-Bstatic" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libstd-5024342751ec4fae.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libpanic_unwind-2ef37a08deacbef7.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libobject-6474163bcabd56d4.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libmemchr-0c669fc4488b33a7.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libaddr2line-facd468809e87d62.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libgimli-a761ff9b49802762.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/librustc_demangle-b5857e32e98a1522.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libstd_detect-b4d4247665203a7e.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libhashbrown-ba5952c0e6997780.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/librustc_std_workspace_alloc-c28e1bddb833f318.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libminiz_oxide-0c142178ac12e90a.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libadler2-9849bba3624604db.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libunwind-91be5c201001b2fd.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libcfg_if-03f10e69535bbda2.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/liblibc-be500544df63862d.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/liballoc-db9414217643e13f.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/librustc_std_workspace_core-fc0ad1732fa36810.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libcore-11d9a250f9da47d5.rlib" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib/libcompiler_builtins-64829956fbeadedf.rlib" "-Wl,-Bdynamic" "-lgcc_s" "-lutil" "-lrt" "-lpthread" "-lm" "-ldl" "-lc" "-L" "/tmp/rustcI2egGS/raw-dylibs" "-B/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld" "-fuse-ld=lld" "-Wl,-znostart-stop-gc" "-Wl,--eh-frame-hdr" "-Wl,-z,noexecstack" "-L" "/home/evan/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/lib" "-o" "main" "-Wl,--gc-sections" "-pie" "-Wl,-z,relro,-z,now" "-nodefaultlibs"
// }

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

        all_bytes.extend_from_slice(&data);
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

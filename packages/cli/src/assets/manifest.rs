use manganis_core::{LinkSection, ResourceAsset};
use object::{read::archive::ArchiveFile, File as ObjectFile, Object, ObjectSection};
use std::{collections::HashMap, path::PathBuf};

use crate::link::InterceptedArgs;

/// A manifest of all assets collected from dependencies
///
/// This will be filled in primarly by incremental compilation artifacts.
#[derive(Debug, PartialEq, Default, Clone)]
pub struct AssetManifest {
    /// Map of asset pathbuf to its
    pub(crate) assets: HashMap<PathBuf, ResourceAsset>,
}

impl AssetManifest {
    /// Creates a new asset manifest
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Fill this manifest from the intercepted rustc args used to link the app together
    pub fn add_from_linker_intercept(&mut self, args: InterceptedArgs) {
        // Attempt to load the arg as a command file, otherwise just use the args themselves
        // This is because windows will pass in `@linkerargs.txt` as a source of linker args
        if let Some(command) = args.args.iter().find(|arg| arg.starts_with('@')).cloned() {
            self.add_from_command_file(args, &command);
        } else {
            self.add_from_linker_args(args);
        }
    }

    /// Fill this manifest from the contents of a linker command file.
    ///
    /// Rustc will pass a file as link args to linkers on windows instead of args directly.
    ///
    /// We actually need to read that file and then pull out the args directly.
    pub fn add_from_command_file(&mut self, args: InterceptedArgs, arg: &str) {
        let path = arg.trim().trim_start_matches('@');
        let file_binary = std::fs::read(path).unwrap();

        // This may be a utf-16le file. Let's try utf-8 first.
        let content = match String::from_utf8(file_binary.clone()) {
            Ok(s) => s,
            Err(_) => {
                // Convert Vec<u8> to Vec<u16> to convert into a String
                let binary_u16le: Vec<u16> = file_binary
                    .chunks_exact(2)
                    .map(|a| u16::from_le_bytes([a[0], a[1]]))
                    .collect();

                String::from_utf16_lossy(&binary_u16le)
            }
        };

        // Gather linker args
        let mut linker_args = Vec::new();
        let lines = content.lines();
        for line in lines {
            // Remove quotes from the line - windows link args files are quoted
            let line_parsed = line.to_string();
            let line_parsed = line_parsed.trim_end_matches('"').to_string();
            let line_parsed = line_parsed.trim_start_matches('"').to_string();
            linker_args.push(line_parsed);
        }

        self.add_from_linker_args(InterceptedArgs {
            args: linker_args,
            ..args
        });
    }

    pub fn add_from_linker_args(&mut self, args: InterceptedArgs) {
        // Parse through linker args for `.o` or `.rlib` files.
        for item in args.args {
            if item.ends_with(".o") || item.ends_with(".rlib") {
                self.add_from_object_path(args.work_dir.join(PathBuf::from(item)));
            }
        }
    }

    /// Fill this manifest with a file object/rlib files, typically extracted from the linker intercepted
    pub fn add_from_object_path(&mut self, path: PathBuf) {
        let Some(ext) = path.extension() else {
            return;
        };

        let Some(ext) = ext.to_str() else {
            return;
        };

        let data = std::fs::read(path.clone()).expect("Failed to read asset optimization file");

        match ext {
            // Parse an unarchived object file
            "o" => {
                let object = object::File::parse(&*data).unwrap();
                self.add_from_object_file(&object);
            }

            // Parse an rlib as a collection of objects
            "rlib" => {
                let archive = object::read::archive::ArchiveFile::parse(&*data).unwrap();
                self.add_from_archive_file(&archive, &data);
            }
            _ => {}
        }
    }

    /// Fill this manifest from an rlib / ar file that contains many object files and their entryies
    pub fn add_from_archive_file(&mut self, archive: &ArchiveFile, data: &[u8]) {
        // Look through each archive member for object files.
        // Read the archive member's binary data (we know it's an object file)
        // And parse it with the normal `object::File::parse` to find the manganis string.
        for member in archive.members() {
            let member = member.unwrap();
            let name = String::from_utf8_lossy(member.name()).to_string();

            // Check if the archive member is an object file and parse it.
            if name.ends_with(".o") {
                let data = member.data(&*data).unwrap();
                let object = object::File::parse(data).unwrap();
                self.add_from_object_file(&object);
            }
        }
    }

    /// Fill this manifest with whatever tables might come from the object file
    pub fn add_from_object_file(&mut self, obj: &ObjectFile) -> Option<()> {
        for section in obj.sections() {
            let Ok(section_name) = section.name() else {
                continue;
            };

            // Check if the link section matches the asset section for one of the platforms we support. This may not be the current platform if the user is cross compiling
            let matches = LinkSection::ALL
                .iter()
                .any(|x| x.link_section == section_name);

            if !matches {
                continue;
            }

            let bytes = section.uncompressed_data().ok()?;

            let as_str = std::str::from_utf8(&bytes)
                .ok()?
                .chars()
                .filter(|c| !c.is_control())
                .collect::<String>();

            let stream = serde_json::Deserializer::from_str(&as_str).into_iter::<ResourceAsset>();

            for as_resource in stream {
                let as_resource = as_resource.unwrap();

                // Some platforms (e.g. macOS) start the manganis section with a null byte, we need to filter that out before we deserialize the JSON
                self.assets
                    .insert(as_resource.absolute.clone(), as_resource);
            }
        }

        None
    }

    /// Copy the assest from this manifest to a target folder
    ///
    /// If `optimize` is enabled, then we will run the optimizer for this asset.
    ///
    /// The output file is guaranteed to be the destination + the ResourceAsset bundle name
    ///
    /// Will not actually copy the asset if the source asset hasn't changed?
    pub fn copy_asset_to(&self, destination: PathBuf, target_asset: PathBuf, optimize: bool) {
        let src = self.assets.get(&target_asset).unwrap();

        let local = src.absolute.clone();

        if !local.exists() {
            panic!("Specified asset does not exist while trying to copy {target_asset:?} to {destination:?}")
        }

        // If there's no optimizaton while copying this asset, we simply std::fs::copy and call it a day
        if !optimize {
            std::fs::copy(local, destination.join(&src.bundled)).expect("Failed to copy asset");
            return;
        }

        // Otherwise, let's attempt to optimize the thing
    }
}

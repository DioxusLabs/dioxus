use manganis_core::LinkSection;
use object::{File, Object, ObjectSection};
use std::fs;
use std::path::PathBuf;

// pub use railwind::warning::Warning as TailwindWarning;
// use crate::{file::process_file, process_folder};
// use manganis_common::{linker, AssetType};

// get the text containing all the asset descriptions
// in the "link section" of the binary
fn get_string_manganis(file: &File) -> Option<String> {
    for section in file.sections() {
        if let Ok(section_name) = section.name() {
            // Check if the link section matches the asset section for one of the platforms we support. This may not be the current platform if the user is cross compiling
            if LinkSection::ALL
                .iter()
                .any(|x| x.link_section == section_name)
            {
                let bytes = section.uncompressed_data().ok()?;
                // Some platforms (e.g. macOS) start the manganis section with a null byte, we need to filter that out before we deserialize the JSON
                return Some(
                    std::str::from_utf8(&bytes)
                        .ok()?
                        .chars()
                        .filter(|c| !c.is_control())
                        .collect::<String>(),
                );
            }
        }
    }
    None
}

/// A manifest of all assets collected from dependencies
#[derive(Debug, PartialEq, Default, Clone)]
pub struct AssetManifest {
    pub(crate) assets: Vec<AssetType>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssetType {
    File(PathBuf),
    Folder(PathBuf),
}

impl AssetManifest {
    /// Creates a new asset manifest
    pub fn new(assets: Vec<AssetType>) -> Self {
        Self { assets }
    }

    /// Returns all assets collected from dependencies
    pub fn assets(&self) -> &Vec<AssetType> {
        &self.assets
    }

    //     /// Returns the HTML that should be injected into the head of the page
    //     pub fn head(&self) -> String {
    //         let mut head = String::new();
    //         for asset in &self.assets {
    //             if let crate::AssetType::Resource(file) = asset {
    //                 match file.options() {
    //                     crate::FileOptions::Css(css_options) => {
    //                         if css_options.preload() {
    //                             if let Ok(asset_path) = file.served_location() {
    //                                 head.push_str(&format!(
    //                                     "<link rel=\"preload\" as=\"style\" href=\"{asset_path}\">\n"
    //                                 ))
    //                             }
    //                         }
    //                     }
    //                     crate::FileOptions::Image(image_options) => {
    //                         if image_options.preload() {
    //                             if let Ok(asset_path) = file.served_location() {
    //                                 head.push_str(&format!(
    //                                     "<link rel=\"preload\" as=\"image\" href=\"{asset_path}\">\n"
    //                                 ))
    //                             }
    //                         }
    //                     }
    //                     crate::FileOptions::Js(js_options) => {
    //                         if js_options.preload() {
    //                             if let Ok(asset_path) = file.served_location() {
    //                                 head.push_str(&format!(
    //                                     "<link rel=\"preload\" as=\"script\" href=\"{asset_path}\">\n"
    //                                 ))
    //                             }
    //                         }
    //                     }
    //                     _ => {}
    //                 }
    //             }
    //         }
    //         head
    //     }
}

// /// An extension trait CLI support for the asset manifest
// pub trait AssetManifestExt {
//     /// Load a manifest from a list of Manganis JSON strings.
//     ///
//     /// The asset descriptions are stored inside a manifest file that is produced when the linker is intercepted.
//     fn load(json: Vec<String>) -> Self;
//     /// Load a manifest from the assets propogated through object files.
//     ///
//     /// The asset descriptions are stored inside a manifest file that is produced when the linker is intercepted.
//     fn load_from_objects(object_paths: Vec<PathBuf>) -> Self;
//     /// Optimize and copy all assets in the manifest to a folder
//     fn copy_static_assets_to(&self, location: impl Into<PathBuf>) -> anyhow::Result<()>;
//     /// Collect all tailwind classes and generate string with the output css
//     fn collect_tailwind_css(
//         &self,
//         include_preflight: bool,
//         warnings: &mut Vec<TailwindWarning>,
//     ) -> String;
// }

// impl AssetManifestExt for AssetManifest {
//     fn load(json: Vec<String>) -> Self {
//         let mut all_assets = Vec::new();

//         // Collect all assets for each manganis string found.
//         for item in json {
//             let mut assets = deserialize_assets(item.as_str());
//             all_assets.append(&mut assets);
//         }

//         // If we don't see any manganis assets used in the binary, just return an empty manifest
//         if all_assets.is_empty() {
//             return Self::default();
//         };

//         Self::new(all_assets)
//     }

//     fn load_from_objects(object_files: Vec<PathBuf>) -> Self {
//         let json = get_json_from_object_files(object_files);
//         Self::load(json)
//     }

//     fn copy_static_assets_to(&self, location: impl Into<PathBuf>) -> anyhow::Result<()> {
//         let location = location.into();
//         match std::fs::create_dir_all(&location) {
//             Ok(_) => {}
//             Err(err) => {
//                 tracing::error!("Failed to create directory for static assets: {}", err);
//                 return Err(err.into());
//             }
//         }

//         self.assets().iter().try_for_each(|asset| {
//             match asset {
//                 AssetType::Resource(file_asset) => {
//                     tracing::info!("Optimizing and bundling {:?}", file_asset);
//                     tracing::trace!("Copying asset from {:?} to {:?}", file_asset, location);
//                     match process_file(file_asset, &location) {
//                         Ok(_) => {}
//                         Err(err) => {
//                             tracing::error!("Failed to copy static asset: {}", err);
//                             return Err(err);
//                         }
//                     }

//                     // tracing::info!("Copying folder asset {}", folder_asset);
//                     // match process_folder(folder_asset, &location) {
//                     //     Ok(_) => {}
//                     //     Err(err) => {
//                     //         tracing::error!("Failed to copy static asset: {}", err);
//                     //         return Err(err);
//                     //     }
//                     // }
//                 }

//                 _ => {}
//             }
//             Ok::<(), anyhow::Error>(())
//         })
//     }

//     // fn collect_tailwind_css(
//     //     self: &AssetManifest,
//     //     include_preflight: bool,
//     //     warnings: &mut Vec<TailwindWarning>,
//     // ) -> String {
//     //     let mut all_classes = String::new();

//     //     for asset in self.assets() {
//     //         if let AssetType::Tailwind(classes) = asset {
//     //             all_classes.push_str(classes.classes());
//     //             all_classes.push(' ');
//     //         }
//     //     }

//     //     let source = railwind::Source::String(all_classes, railwind::CollectionOptions::String);

//     //     let css = railwind::parse_to_string(source, include_preflight, warnings);

//     //     crate::file::minify_css(&css)
//     // }
// }

fn deserialize_assets(json: &str) -> Vec<AssetType> {
    todo!()
    // let deserializer = serde_json::Deserializer::from_str(json);
    // deserializer
    //     .into_iter::<AssetType>()
    //     .flat_map(|x| x.ok())
    //     // .map(|x| x.unwrap())
    //     .collect()
}

/// Extract JSON Manganis strings from a list of object files.
pub fn get_json_from_object_files(object_paths: Vec<PathBuf>) -> Vec<String> {
    let mut all_json = Vec::new();

    for path in object_paths {
        let Some(ext) = path.extension() else {
            continue;
        };

        let Some(ext) = ext.to_str() else {
            continue;
        };

        let is_rlib = match ext {
            "rlib" => true,
            "o" => false,
            _ => continue,
        };

        // Read binary data and try getting assets from manganis string
        let binary_data = fs::read(path).unwrap();

        // rlibs are archives with object files inside.
        let mut data = match is_rlib {
            false => {
                // Parse an unarchived object file. We use a Vec to match the return types.
                let file = object::File::parse(&*binary_data).unwrap();
                let mut data = Vec::new();
                if let Some(string) = get_string_manganis(&file) {
                    data.push(string);
                }
                data
            }
            true => {
                let file = object::read::archive::ArchiveFile::parse(&*binary_data).unwrap();

                // rlibs can contain many object files so we collect each manganis string here.
                let mut manganis_strings = Vec::new();

                // Look through each archive member for object files.
                // Read the archive member's binary data (we know it's an object file)
                // And parse it with the normal `object::File::parse` to find the manganis string.
                for member in file.members() {
                    let member = member.unwrap();
                    let name = String::from_utf8_lossy(member.name()).to_string();

                    // Check if the archive member is an object file and parse it.
                    if name.ends_with(".o") {
                        let data = member.data(&*binary_data).unwrap();
                        let o_file = object::File::parse(data).unwrap();
                        if let Some(manganis_str) = get_string_manganis(&o_file) {
                            manganis_strings.push(manganis_str);
                        }
                    }
                }

                manganis_strings
            }
        };

        all_json.append(&mut data);
    }

    all_json
}

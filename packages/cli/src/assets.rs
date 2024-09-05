use crate::Result;
use anyhow::Context;
use brotli::enc::BrotliEncoderParams;
use manganis_core::{LinkSection, ResourceAsset};
use object::{read::archive::ArchiveFile, File as ObjectFile, Object, ObjectSection};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};
use std::{fs::File, io::Write};
use tracing::Level;
use walkdir::WalkDir;
// use manganis_common::{FileOptions, FolderAsset};
// use image::{DynamicImage, EncodableLayout};
// use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
// use manganis_common::{
//     CssOptions, FileOptions, ImageOptions, ImageType, JsOptions, JsonOptions, ResourceAsset,
// };

use crate::link::InterceptedArgs;

/// A manifest of all assets collected from dependencies
///
/// This will be filled in primarly by incremental compilation artifacts.
#[derive(Debug, PartialEq, Default, Clone)]
pub(crate) struct AssetManifest {
    pub(crate) assets: HashMap<PathBuf, ResourceAsset>,
}

impl AssetManifest {
    /// Create a new asset manifest pre-populated with the assets from the linker intercept
    pub(crate) fn new_from_linker_intercept(args: InterceptedArgs) -> Self {
        let mut manifest = Self::default();
        manifest.add_from_linker_intercept(args);
        manifest
    }

    /// Fill this manifest from the intercepted rustc args used to link the app together
    pub(crate) fn add_from_linker_intercept(&mut self, args: InterceptedArgs) {
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
    pub(crate) fn add_from_command_file(&mut self, args: InterceptedArgs, arg: &str) {
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

    pub(crate) fn add_from_linker_args(&mut self, args: InterceptedArgs) {
        // Parse through linker args for `.o` or `.rlib` files.
        for item in args.args {
            if item.ends_with(".o") || item.ends_with(".rlib") {
                self.add_from_object_path(args.work_dir.join(PathBuf::from(item)));
            }
        }
    }

    /// Fill this manifest with a file object/rlib files, typically extracted from the linker intercepted
    pub(crate) fn add_from_object_path(&mut self, path: PathBuf) {
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
    pub(crate) fn add_from_archive_file(&mut self, archive: &ArchiveFile, data: &[u8]) {
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
    pub(crate) fn add_from_object_file(&mut self, obj: &ObjectFile) -> Option<()> {
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
    pub(crate) fn copy_asset_to(
        &self,
        destination: &Path,
        target_asset: &Path,
        optimize: bool,
        _pre_compress: bool,
    ) -> anyhow::Result<()> {
        let Some(src) = self.assets.get(target_asset) else {
            crate::fastfs::copy_asset(
                target_asset,
                &destination.join(target_asset.file_name().unwrap()),
            )?;

            return Ok(());
        };

        let local = src.absolute.clone();

        if !local.exists() {
            panic!("Specified asset does not exist while trying to copy {target_asset:?} to {destination:?}")
        }

        // If there's no optimizaton while copying this asset, we simply std::fs::copy and call it a day
        if !optimize {
            std::fs::copy(local, destination.join(&src.bundled)).expect("Failed to copy asset");
            return Ok(());
        }

        // Otherwise, let's attempt to optimize the the asset we're copying

        Ok(())
    }
}

// use swc::{config::JsMinifyOptions, try_with_handler, BoolOrDataConfig};
// use swc_common::{sync::Lrc, FileName};
// use swc_common::{SourceMap, GLOBALS};

// pub(crate) trait Process {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()>;
// }

// /// Process a specific file asset
// pub(crate) fn process_file(file: &ResourceAsset, output_folder: &Path) -> anyhow::Result<()> {
//     todo!()
//     // let location = file.location();
//     // let source = location.source();
//     // let output_path = output_folder.join(location.unique_name());
//     // file.options().process(source, &output_path)
// }

// impl Process for FileOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         if output_path.exists() {
//             return Ok(());
//         }
//         match self {
//             Self::Other { .. } => {
//                 let bytes = source.read_to_bytes()?;
//                 std::fs::write(output_path, bytes).with_context(|| {
//                     format!(
//                         "Failed to write file to output location: {}",
//                         output_path.display()
//                     )
//                 })?;
//             }
//             Self::Css(options) => {
//                 options.process(source, output_path)?;
//             }
//             Self::Js(options) => {
//                 options.process(source, output_path)?;
//             }
//             Self::Json(options) => {
//                 options.process(source, output_path)?;
//             }
//             Self::Image(options) => {
//                 options.process(source, output_path)?;
//             }
//             _ => todo!(),
//         }

//         Ok(())
//     }
// }

// impl Process for ImageOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         let mut image = image::ImageReader::new(std::io::Cursor::new(&*source.read_to_bytes()?))
//             .with_guessed_format()?
//             .decode()?;

//         if let Some(size) = self.size() {
//             image = image.resize_exact(size.0, size.1, image::imageops::FilterType::Lanczos3);
//         }

//         match self.ty() {
//             ImageType::Png => {
//                 compress_png(image, output_path);
//             }
//             ImageType::Jpg => {
//                 compress_jpg(image, output_path)?;
//             }
//             ImageType::Avif => {
//                 if let Err(error) = image.save(output_path) {
//                     tracing::error!("Failed to save avif image: {} with path {}. You must have the avif feature enabled to use avif assets", error, output_path.display());
//                 }
//             }
//             ImageType::Webp => {
//                 if let Err(err) = image.save(output_path) {
//                     tracing::error!("Failed to save webp image: {}. You must have the avif feature enabled to use webp assets", err);
//                 }
//             }
//         }

//         Ok(())
//     }
// }

// fn compress_jpg(image: DynamicImage, output_location: &Path) -> anyhow::Result<()> {
//     let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_EXT_RGBX);
//     let width = image.width() as usize;
//     let height = image.height() as usize;

//     comp.set_size(width, height);
//     let mut comp = comp.start_compress(Vec::new())?; // any io::Write will work

//     comp.write_scanlines(image.to_rgba8().as_bytes())?;

//     let jpeg_bytes = comp.finish()?;

//     let file = std::fs::File::create(output_location)?;
//     let w = &mut BufWriter::new(file);
//     w.write_all(&jpeg_bytes)?;
//     Ok(())
// }

// fn compress_png(image: DynamicImage, output_location: &Path) {
//     // Image loading/saving is outside scope of this library
//     let width = image.width() as usize;
//     let height = image.height() as usize;
//     let bitmap: Vec<_> = image
//         .into_rgba8()
//         .pixels()
//         .map(|px| imagequant::RGBA::new(px[0], px[1], px[2], px[3]))
//         .collect();

//     // Configure the library
//     let mut liq = imagequant::new();
//     liq.set_speed(5).unwrap();
//     liq.set_quality(0, 99).unwrap();

//     // Describe the bitmap
//     let mut img = liq.new_image(&bitmap[..], width, height, 0.0).unwrap();

//     // The magic happens in quantize()
//     let mut res = match liq.quantize(&mut img) {
//         Ok(res) => res,
//         Err(err) => panic!("Quantization failed, because: {err:?}"),
//     };

//     let (palette, pixels) = res.remapped(&mut img).unwrap();

//     let file = std::fs::File::create(output_location).unwrap();
//     let w = &mut BufWriter::new(file);

//     let mut encoder = png::Encoder::new(w, width as u32, height as u32);
//     encoder.set_color(png::ColorType::Rgba);
//     let mut flattened_palette = Vec::new();
//     let mut alpha_palette = Vec::new();
//     for px in palette {
//         flattened_palette.push(px.r);
//         flattened_palette.push(px.g);
//         flattened_palette.push(px.b);
//         alpha_palette.push(px.a);
//     }
//     encoder.set_palette(flattened_palette);
//     encoder.set_trns(alpha_palette);
//     encoder.set_depth(png::BitDepth::Eight);
//     encoder.set_color(png::ColorType::Indexed);
//     encoder.set_compression(png::Compression::Best);
//     let mut writer = encoder.write_header().unwrap();
//     writer.write_image_data(&pixels).unwrap();
//     writer.finish().unwrap();
// }

// impl Process for CssOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         let css = source.read_to_string()?;

//         let css = if self.minify() { minify_css(&css) } else { css };

//         std::fs::write(output_path, css).with_context(|| {
//             format!(
//                 "Failed to write css to output location: {}",
//                 output_path.display()
//             )
//         })?;

//         Ok(())
//     }
// }

// pub(crate) fn minify_css(css: &str) -> String {
//     let mut stylesheet = StyleSheet::parse(css, ParserOptions::default()).unwrap();
//     stylesheet.minify(MinifyOptions::default()).unwrap();
//     let printer = PrinterOptions {
//         minify: true,
//         ..Default::default()
//     };
//     let res = stylesheet.to_css(printer).unwrap();
//     res.code
// }

// pub(crate) fn minify_js(source: &ResourceAsset) -> anyhow::Result<String> {
//     todo!("disabled swc due to semver issues")
//     // let cm = Arc::<SourceMap>::default();

//     // let js = source.read_to_string()?;
//     // let c = swc::Compiler::new(cm.clone());
//     // let output = GLOBALS
//     //     .set(&Default::default(), || {
//     //         try_with_handler(cm.clone(), Default::default(), |handler| {
//     //             // let filename = Lrc::new(match source {
//     //             //     manganis_common::ResourceAsset::Local(path) => {
//     //             //         FileName::Real(path.canonicalized.clone())
//     //             //     }
//     //             //     manganis_common::ResourceAsset::Remote(url) => FileName::Url(url.clone()),
//     //             // });
//     //             let filename = todo!();
//     //             let fm = cm.new_source_file(filename, js.to_string());

//     //             c.minify(
//     //                 fm,
//     //                 handler,
//     //                 &JsMinifyOptions {
//     //                     compress: BoolOrDataConfig::from_bool(true),
//     //                     mangle: BoolOrDataConfig::from_bool(true),
//     //                     ..Default::default()
//     //                 },
//     //             )
//     //             .context("failed to minify javascript")
//     //         })
//     //     })
//     //     .map(|output| output.code);

//     // match output {
//     //     Ok(output) => Ok(output),
//     //     Err(err) => {
//     //         tracing::error!("Failed to minify javascript: {}", err);
//     //         Ok(js)
//     //     }
//     // }
// }

// impl Process for JsOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         let js = if self.minify() {
//             minify_js(source)?
//         } else {
//             source.read_to_string()?
//         };

//         std::fs::write(output_path, js).with_context(|| {
//             format!(
//                 "Failed to write js to output location: {}",
//                 output_path.display()
//             )
//         })?;

//         Ok(())
//     }
// }

// pub(crate) fn minify_json(source: &str) -> anyhow::Result<String> {
//     // First try to parse the json
//     let json: serde_json::Value = serde_json::from_str(source)?;
//     // Then print it in a minified format
//     let json = serde_json::to_string(&json)?;
//     Ok(json)
// }

// impl Process for JsonOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         let source = source.read_to_string()?;
//         let json = match minify_json(&source) {
//             Ok(json) => json,
//             Err(err) => {
//                 tracing::error!("Failed to minify json: {}", err);
//                 source
//             }
//         };

//         std::fs::write(output_path, json).with_context(|| {
//             format!(
//                 "Failed to write json to output location: {}",
//                 output_path.display()
//             )
//         })?;

//         Ok(())
//     }
// }

// /// Process a folder, optimizing and copying all assets into the output folder
// pub(crate) fn process_folder(folder: &FolderAsset, output_folder: &Path) -> anyhow::Result<()> {
//     // Push the unique name of the folder to the output folder
//     let output_folder = output_folder.join(folder.unique_name());

//     if output_folder.exists() {
//         return Ok(());
//     }

//     // .location()
//     // // .source()
//     // .as_path()
//     let folder = folder.path();

//     // Optimize and copy all assets in the folder in parallel
//     process_folder_inner(folder, &output_folder)
// }

// fn process_folder_inner(folder: &Path, output_folder: &Path) -> anyhow::Result<()> {
//     // Create the folder
//     std::fs::create_dir_all(output_folder)?;

//     // Then optimize children
//     let files: Vec<_> = std::fs::read_dir(folder)
//         .into_iter()
//         .flatten()
//         .flatten()
//         .collect();

//     files.par_iter().try_for_each(|file| {
//         let file = file.path();
//         let metadata = file.metadata()?;
//         let output_path = output_folder.join(file.strip_prefix(folder)?);
//         if metadata.is_dir() {
//             process_folder_inner(&file, &output_path)
//         } else {
//             process_file_minimal(&file, &output_path)
//         }
//     })?;

//     Ok(())
// }

// /// Optimize a file without changing any of its contents significantly (e.g. by changing the extension)
// fn process_file_minimal(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
//     todo!()
//     // let options =
//     //     FileOptions::default_for_extension(input_path.extension().and_then(|e| e.to_str()));
//     // let source = input_path.to_path_buf();
//     // options.process(&source, output_path)?;
//     // Ok(())
// }

// use image::{DynamicImage, EncodableLayout};
// use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
// use manganis_common::{
//     CssOptions, FileOptions, ImageOptions, ImageType, JsOptions, JsonOptions, ResourceAsset,
// };

// use swc::{config::JsMinifyOptions, try_with_handler, BoolOrDataConfig};
// use swc_common::{sync::Lrc, FileName};
// use swc_common::{SourceMap, GLOBALS};

// pub(crate) trait Process {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()>;
// }

// /// Process a specific file asset
// pub(crate) fn process_file(file: &ResourceAsset, output_folder: &Path) -> anyhow::Result<()> {
//     todo!()
//     // let location = file.location();
//     // let source = location.source();
//     // let output_path = output_folder.join(location.unique_name());
//     // file.options().process(source, &output_path)
// }

// impl Process for FileOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         if output_path.exists() {
//             return Ok(());
//         }
//         match self {
//             Self::Other { .. } => {
//                 let bytes = source.read_to_bytes()?;
//                 std::fs::write(output_path, bytes).with_context(|| {
//                     format!(
//                         "Failed to write file to output location: {}",
//                         output_path.display()
//                     )
//                 })?;
//             }
//             Self::Css(options) => {
//                 options.process(source, output_path)?;
//             }
//             Self::Js(options) => {
//                 options.process(source, output_path)?;
//             }
//             Self::Json(options) => {
//                 options.process(source, output_path)?;
//             }
//             Self::Image(options) => {
//                 options.process(source, output_path)?;
//             }
//             _ => todo!(),
//         }

//         Ok(())
//     }
// }

// impl Process for ImageOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         let mut image = image::ImageReader::new(std::io::Cursor::new(&*source.read_to_bytes()?))
//             .with_guessed_format()?
//             .decode()?;

//         if let Some(size) = self.size() {
//             image = image.resize_exact(size.0, size.1, image::imageops::FilterType::Lanczos3);
//         }

//         match self.ty() {
//             ImageType::Png => {
//                 compress_png(image, output_path);
//             }
//             ImageType::Jpg => {
//                 compress_jpg(image, output_path)?;
//             }
//             ImageType::Avif => {
//                 if let Err(error) = image.save(output_path) {
//                     tracing::error!("Failed to save avif image: {} with path {}. You must have the avif feature enabled to use avif assets", error, output_path.display());
//                 }
//             }
//             ImageType::Webp => {
//                 if let Err(err) = image.save(output_path) {
//                     tracing::error!("Failed to save webp image: {}. You must have the avif feature enabled to use webp assets", err);
//                 }
//             }
//         }

//         Ok(())
//     }
// }

// fn compress_jpg(image: DynamicImage, output_location: &Path) -> anyhow::Result<()> {
//     let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_EXT_RGBX);
//     let width = image.width() as usize;
//     let height = image.height() as usize;

//     comp.set_size(width, height);
//     let mut comp = comp.start_compress(Vec::new())?; // any io::Write will work

//     comp.write_scanlines(image.to_rgba8().as_bytes())?;

//     let jpeg_bytes = comp.finish()?;

//     let file = std::fs::File::create(output_location)?;
//     let w = &mut BufWriter::new(file);
//     w.write_all(&jpeg_bytes)?;
//     Ok(())
// }

// fn compress_png(image: DynamicImage, output_location: &Path) {
//     // Image loading/saving is outside scope of this library
//     let width = image.width() as usize;
//     let height = image.height() as usize;
//     let bitmap: Vec<_> = image
//         .into_rgba8()
//         .pixels()
//         .map(|px| imagequant::RGBA::new(px[0], px[1], px[2], px[3]))
//         .collect();

//     // Configure the library
//     let mut liq = imagequant::new();
//     liq.set_speed(5).unwrap();
//     liq.set_quality(0, 99).unwrap();

//     // Describe the bitmap
//     let mut img = liq.new_image(&bitmap[..], width, height, 0.0).unwrap();

//     // The magic happens in quantize()
//     let mut res = match liq.quantize(&mut img) {
//         Ok(res) => res,
//         Err(err) => panic!("Quantization failed, because: {err:?}"),
//     };

//     let (palette, pixels) = res.remapped(&mut img).unwrap();

//     let file = std::fs::File::create(output_location).unwrap();
//     let w = &mut BufWriter::new(file);

//     let mut encoder = png::Encoder::new(w, width as u32, height as u32);
//     encoder.set_color(png::ColorType::Rgba);
//     let mut flattened_palette = Vec::new();
//     let mut alpha_palette = Vec::new();
//     for px in palette {
//         flattened_palette.push(px.r);
//         flattened_palette.push(px.g);
//         flattened_palette.push(px.b);
//         alpha_palette.push(px.a);
//     }
//     encoder.set_palette(flattened_palette);
//     encoder.set_trns(alpha_palette);
//     encoder.set_depth(png::BitDepth::Eight);
//     encoder.set_color(png::ColorType::Indexed);
//     encoder.set_compression(png::Compression::Best);
//     let mut writer = encoder.write_header().unwrap();
//     writer.write_image_data(&pixels).unwrap();
//     writer.finish().unwrap();
// }

// impl Process for CssOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         let css = source.read_to_string()?;

//         let css = if self.minify() { minify_css(&css) } else { css };

//         std::fs::write(output_path, css).with_context(|| {
//             format!(
//                 "Failed to write css to output location: {}",
//                 output_path.display()
//             )
//         })?;

//         Ok(())
//     }
// }

// pub(crate) fn minify_css(css: &str) -> String {
//     let mut stylesheet = StyleSheet::parse(css, ParserOptions::default()).unwrap();
//     stylesheet.minify(MinifyOptions::default()).unwrap();
//     let printer = PrinterOptions {
//         minify: true,
//         ..Default::default()
//     };
//     let res = stylesheet.to_css(printer).unwrap();
//     res.code
// }

// pub(crate) fn minify_js(source: &ResourceAsset) -> anyhow::Result<String> {
//     todo!("disabled swc due to semver issues")
//     // let cm = Arc::<SourceMap>::default();

//     // let js = source.read_to_string()?;
//     // let c = swc::Compiler::new(cm.clone());
//     // let output = GLOBALS
//     //     .set(&Default::default(), || {
//     //         try_with_handler(cm.clone(), Default::default(), |handler| {
//     //             // let filename = Lrc::new(match source {
//     //             //     manganis_common::ResourceAsset::Local(path) => {
//     //             //         FileName::Real(path.canonicalized.clone())
//     //             //     }
//     //             //     manganis_common::ResourceAsset::Remote(url) => FileName::Url(url.clone()),
//     //             // });
//     //             let filename = todo!();
//     //             let fm = cm.new_source_file(filename, js.to_string());

//     //             c.minify(
//     //                 fm,
//     //                 handler,
//     //                 &JsMinifyOptions {
//     //                     compress: BoolOrDataConfig::from_bool(true),
//     //                     mangle: BoolOrDataConfig::from_bool(true),
//     //                     ..Default::default()
//     //                 },
//     //             )
//     //             .context("failed to minify javascript")
//     //         })
//     //     })
//     //     .map(|output| output.code);

//     // match output {
//     //     Ok(output) => Ok(output),
//     //     Err(err) => {
//     //         tracing::error!("Failed to minify javascript: {}", err);
//     //         Ok(js)
//     //     }
//     // }
// }

// impl Process for JsOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         let js = if self.minify() {
//             minify_js(source)?
//         } else {
//             source.read_to_string()?
//         };

//         std::fs::write(output_path, js).with_context(|| {
//             format!(
//                 "Failed to write js to output location: {}",
//                 output_path.display()
//             )
//         })?;

//         Ok(())
//     }
// }

// pub(crate) fn minify_json(source: &str) -> anyhow::Result<String> {
//     // First try to parse the json
//     let json: serde_json::Value = serde_json::from_str(source)?;
//     // Then print it in a minified format
//     let json = serde_json::to_string(&json)?;
//     Ok(json)
// }

// impl Process for JsonOptions {
//     fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
//         let source = source.read_to_string()?;
//         let json = match minify_json(&source) {
//             Ok(json) => json,
//             Err(err) => {
//                 tracing::error!("Failed to minify json: {}", err);
//                 source
//             }
//         };

//         std::fs::write(output_path, json).with_context(|| {
//             format!(
//                 "Failed to write json to output location: {}",
//                 output_path.display()
//             )
//         })?;

//         Ok(())
//     }
// }

//     /// Returns the HTML that should be injected into the head of the page
//     pub(crate) fn head(&self) -> String {
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

// /// An extension trait CLI support for the asset manifest
// pub(crate) trait AssetManifestExt {
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

// The temp file name for passing manganis json from linker to current exec.
// pub(crate) const MG_JSON_OUT: &str = "mg-out";

// /// Create a head file that contains all of the imports for assets that the user project uses
// pub(crate) fn create_assets_head(build: &BuildRequest, manifest: &AssetManifest) -> Result<()> {
//     let out_dir = build.target_out_dir();
//     std::fs::create_dir_all(&out_dir)?;
//     let mut file = File::create(out_dir.join("__assets_head.html"))?;
//     file.write_all(manifest.head().as_bytes())?;
//     Ok(())
// }

// use crate::file::Process;

// /// Process a folder, optimizing and copying all assets into the output folder
// pub(crate) fn process_folder(folder: &FolderAsset, output_folder: &Path) -> anyhow::Result<()> {
//     // Push the unique name of the folder to the output folder
//     let output_folder = output_folder.join(folder.unique_name());

//     if output_folder.exists() {
//         return Ok(());
//     }

//     // .location()
//     // // .source()
//     // .as_path()
//     let folder = folder.path();

//     // Optimize and copy all assets in the folder in parallel
//     process_folder_inner(folder, &output_folder)
// }

// fn process_folder_inner(folder: &Path, output_folder: &Path) -> anyhow::Result<()> {
//     // Create the folder
//     std::fs::create_dir_all(output_folder)?;

//     // Then optimize children
//     let files: Vec<_> = std::fs::read_dir(folder)
//         .into_iter()
//         .flatten()
//         .flatten()
//         .collect();

//     files.par_iter().try_for_each(|file| {
//         let file = file.path();
//         let metadata = file.metadata()?;
//         let output_path = output_folder.join(file.strip_prefix(folder)?);
//         if metadata.is_dir() {
//             process_folder_inner(&file, &output_path)
//         } else {
//             process_file_minimal(&file, &output_path)
//         }
//     })?;

//     Ok(())
// }

// /// Optimize a file without changing any of its contents significantly (e.g. by changing the extension)
// fn process_file_minimal(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
//     todo!()
//     // let options =
//     //     FileOptions::default_for_extension(input_path.extension().and_then(|e| e.to_str()));
//     // let source = input_path.to_path_buf();
//     // options.process(&source, output_path)?;
//     // Ok(())
// }

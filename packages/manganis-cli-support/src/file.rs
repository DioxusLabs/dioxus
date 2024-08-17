use anyhow::Context;
use image::{DynamicImage, EncodableLayout};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use manganis_common::{
    CssOptions, FileOptions, ImageOptions, ImageType, JsOptions, JsonOptions, ResourceAsset,
};
use std::{
    io::{BufWriter, Write},
    path::Path,
    sync::Arc,
};
use swc::{config::JsMinifyOptions, try_with_handler, BoolOrDataConfig};
use swc_common::{sync::Lrc, FileName};
use swc_common::{SourceMap, GLOBALS};

pub trait Process {
    fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()>;
}

/// Process a specific file asset
pub fn process_file(file: &ResourceAsset, output_folder: &Path) -> anyhow::Result<()> {
    todo!()
    // let location = file.location();
    // let source = location.source();
    // let output_path = output_folder.join(location.unique_name());
    // file.options().process(source, &output_path)
}

impl Process for FileOptions {
    fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
        if output_path.exists() {
            return Ok(());
        }
        match self {
            Self::Other { .. } => {
                let bytes = source.read_to_bytes()?;
                std::fs::write(output_path, bytes).with_context(|| {
                    format!(
                        "Failed to write file to output location: {}",
                        output_path.display()
                    )
                })?;
            }
            Self::Css(options) => {
                options.process(source, output_path)?;
            }
            Self::Js(options) => {
                options.process(source, output_path)?;
            }
            Self::Json(options) => {
                options.process(source, output_path)?;
            }
            Self::Image(options) => {
                options.process(source, output_path)?;
            }
            _ => todo!(),
        }

        Ok(())
    }
}

impl Process for ImageOptions {
    fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
        let mut image = image::ImageReader::new(std::io::Cursor::new(&*source.read_to_bytes()?))
            .with_guessed_format()?
            .decode()?;

        if let Some(size) = self.size() {
            image = image.resize_exact(size.0, size.1, image::imageops::FilterType::Lanczos3);
        }

        match self.ty() {
            ImageType::Png => {
                compress_png(image, output_path);
            }
            ImageType::Jpg => {
                compress_jpg(image, output_path)?;
            }
            ImageType::Avif => {
                if let Err(error) = image.save(output_path) {
                    tracing::error!("Failed to save avif image: {} with path {}. You must have the avif feature enabled to use avif assets", error, output_path.display());
                }
            }
            ImageType::Webp => {
                if let Err(err) = image.save(output_path) {
                    tracing::error!("Failed to save webp image: {}. You must have the avif feature enabled to use webp assets", err);
                }
            }
        }

        Ok(())
    }
}

fn compress_jpg(image: DynamicImage, output_location: &Path) -> anyhow::Result<()> {
    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_EXT_RGBX);
    let width = image.width() as usize;
    let height = image.height() as usize;

    comp.set_size(width, height);
    let mut comp = comp.start_compress(Vec::new())?; // any io::Write will work

    comp.write_scanlines(image.to_rgba8().as_bytes())?;

    let jpeg_bytes = comp.finish()?;

    let file = std::fs::File::create(output_location)?;
    let w = &mut BufWriter::new(file);
    w.write_all(&jpeg_bytes)?;
    Ok(())
}

fn compress_png(image: DynamicImage, output_location: &Path) {
    // Image loading/saving is outside scope of this library
    let width = image.width() as usize;
    let height = image.height() as usize;
    let bitmap: Vec<_> = image
        .into_rgba8()
        .pixels()
        .map(|px| imagequant::RGBA::new(px[0], px[1], px[2], px[3]))
        .collect();

    // Configure the library
    let mut liq = imagequant::new();
    liq.set_speed(5).unwrap();
    liq.set_quality(0, 99).unwrap();

    // Describe the bitmap
    let mut img = liq.new_image(&bitmap[..], width, height, 0.0).unwrap();

    // The magic happens in quantize()
    let mut res = match liq.quantize(&mut img) {
        Ok(res) => res,
        Err(err) => panic!("Quantization failed, because: {err:?}"),
    };

    let (palette, pixels) = res.remapped(&mut img).unwrap();

    let file = std::fs::File::create(output_location).unwrap();
    let w = &mut BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    let mut flattened_palette = Vec::new();
    let mut alpha_palette = Vec::new();
    for px in palette {
        flattened_palette.push(px.r);
        flattened_palette.push(px.g);
        flattened_palette.push(px.b);
        alpha_palette.push(px.a);
    }
    encoder.set_palette(flattened_palette);
    encoder.set_trns(alpha_palette);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_compression(png::Compression::Best);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
}

impl Process for CssOptions {
    fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
        let css = source.read_to_string()?;

        let css = if self.minify() { minify_css(&css) } else { css };

        std::fs::write(output_path, css).with_context(|| {
            format!(
                "Failed to write css to output location: {}",
                output_path.display()
            )
        })?;

        Ok(())
    }
}

pub(crate) fn minify_css(css: &str) -> String {
    let mut stylesheet = StyleSheet::parse(css, ParserOptions::default()).unwrap();
    stylesheet.minify(MinifyOptions::default()).unwrap();
    let printer = PrinterOptions {
        minify: true,
        ..Default::default()
    };
    let res = stylesheet.to_css(printer).unwrap();
    res.code
}

pub(crate) fn minify_js(source: &ResourceAsset) -> anyhow::Result<String> {
    let cm = Arc::<SourceMap>::default();

    let js = source.read_to_string()?;
    let c = swc::Compiler::new(cm.clone());
    let output = GLOBALS
        .set(&Default::default(), || {
            try_with_handler(cm.clone(), Default::default(), |handler| {
                // let filename = Lrc::new(match source {
                //     manganis_common::ResourceAsset::Local(path) => {
                //         FileName::Real(path.canonicalized.clone())
                //     }
                //     manganis_common::ResourceAsset::Remote(url) => FileName::Url(url.clone()),
                // });
                let filename = todo!();
                let fm = cm.new_source_file(filename, js.to_string());

                c.minify(
                    fm,
                    handler,
                    &JsMinifyOptions {
                        compress: BoolOrDataConfig::from_bool(true),
                        mangle: BoolOrDataConfig::from_bool(true),
                        ..Default::default()
                    },
                )
                .context("failed to minify javascript")
            })
        })
        .map(|output| output.code);

    match output {
        Ok(output) => Ok(output),
        Err(err) => {
            tracing::error!("Failed to minify javascript: {}", err);
            Ok(js)
        }
    }
}

impl Process for JsOptions {
    fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
        let js = if self.minify() {
            minify_js(source)?
        } else {
            source.read_to_string()?
        };

        std::fs::write(output_path, js).with_context(|| {
            format!(
                "Failed to write js to output location: {}",
                output_path.display()
            )
        })?;

        Ok(())
    }
}

pub(crate) fn minify_json(source: &str) -> anyhow::Result<String> {
    // First try to parse the json
    let json: serde_json::Value = serde_json::from_str(source)?;
    // Then print it in a minified format
    let json = serde_json::to_string(&json)?;
    Ok(json)
}

impl Process for JsonOptions {
    fn process(&self, source: &ResourceAsset, output_path: &Path) -> anyhow::Result<()> {
        let source = source.read_to_string()?;
        let json = match minify_json(&source) {
            Ok(json) => json,
            Err(err) => {
                tracing::error!("Failed to minify json: {}", err);
                source
            }
        };

        std::fs::write(output_path, json).with_context(|| {
            format!(
                "Failed to write json to output location: {}",
                output_path.display()
            )
        })?;

        Ok(())
    }
}

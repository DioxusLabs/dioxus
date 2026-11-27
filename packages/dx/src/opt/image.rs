use anyhow::Context;
use image::{DynamicImage, EncodableLayout};
use manganis_core::{ImageAssetOptions, ImageFormat, ImageSize};
use std::io::{BufWriter, Write};
use std::path::Path;

/// The dimension that follows a fixed one, rounded to the nearest pixel and never zero.
fn derived_dimension(fixed: u32, source_fixed: u32, source_other: u32) -> u32 {
    if source_fixed == 0 {
        return 1;
    }
    ((source_other as u64 * fixed as u64 + source_fixed as u64 / 2) / source_fixed as u64).max(1)
        as u32
}

pub(crate) fn process_image(
    image_options: &ImageAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    let mut image = image::ImageReader::new(std::io::Cursor::new(&*std::fs::read(source)?))
        .with_guessed_format()
        .context("Failed to guess image format")?
        .decode();

    if let Ok(image) = &mut image {
        let size = match image_options.size() {
            ImageSize::Manual { width, height } => Some((width, height)),
            ImageSize::Width(width) => Some((
                width,
                derived_dimension(width, image.width(), image.height()),
            )),
            ImageSize::Height(height) => Some((
                derived_dimension(height, image.height(), image.width()),
                height,
            )),
            ImageSize::Automatic => None,
        };
        if let Some((width, height)) = size {
            *image = image.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        }
    }

    match (image, image_options.format()) {
        (image, ImageFormat::Png) => {
            compress_png(image.context("Failed to decode image")?, output_path);
        }
        (image, ImageFormat::Jpg) => {
            compress_jpg(image.context("Failed to decode image")?, output_path)?;
        }
        (Ok(image), ImageFormat::Avif) => {
            if let Err(error) = image.save(output_path) {
                tracing::error!(
                    "Failed to save avif image: {} with path {}. You must have the avif feature enabled to use avif assets",
                    error,
                    output_path.display()
                );
            }
        }
        (Ok(image), ImageFormat::Webp) => {
            if let Err(err) = image.save(output_path) {
                tracing::error!(
                    "Failed to save webp image: {}. You must have the avif feature enabled to use webp assets",
                    err
                );
            }
        }
        (Ok(image), _) => {
            image.save(output_path).with_context(|| {
                format!(
                    "Failed to save image (from {}) with path {}",
                    source.display(),
                    output_path.display()
                )
            })?;
        }
        // If we can't decode the image or it is of an unknown type, we just copy the file
        _ => {
            let source_file = std::fs::File::open(source).context("Failed to open source file")?;
            let mut reader = std::io::BufReader::new(source_file);
            let output_file = std::fs::File::create(output_path).with_context(|| {
                format!("Failed to create output file: {}", output_path.display())
            })?;
            let mut writer = std::io::BufWriter::new(output_file);
            std::io::copy(&mut reader, &mut writer)
                .with_context(|| {
                    format!(
                        "Failed to write image to output location: {}",
                        output_path.display()
                    )
                })
                .context("Failed to copy image data")?;
        }
    }

    Ok(())
}

pub(crate) fn compress_png(image: DynamicImage, output_location: &Path) {
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

pub(crate) fn compress_jpg(image: DynamicImage, output_location: &Path) -> anyhow::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use manganis_core::{AssetOptions, AssetVariant};

    /// A 400x200 source, so a derived dimension is unambiguous.
    fn processed_size(size: ImageSize) -> (u32, u32) {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.png");
        RgbaImage::from_pixel(400, 200, Rgba([255, 0, 0, 255]))
            .save(&source)
            .unwrap();
        let output = dir.path().join("output.png");

        let options = AssetOptions::image().with_size(size).into_asset_options();
        let AssetVariant::Image(image_options) = options.variant() else {
            panic!("image options expected");
        };

        process_image(image_options, &source, &output).unwrap();
        let image = image::open(output).unwrap();
        (image.width(), image.height())
    }

    /// Given an image with only a width, when it is processed, then the height follows the source aspect ratio.
    #[test]
    fn a_width_only_size_derives_the_height() {
        assert_eq!(processed_size(ImageSize::Width(100)), (100, 50));
    }

    /// Given an image with only a height, when it is processed, then the width follows the source aspect ratio.
    #[test]
    fn a_height_only_size_derives_the_width() {
        assert_eq!(processed_size(ImageSize::Height(50)), (100, 50));
    }

    /// Given an image with both dimensions, when it is processed, then the exact size is kept.
    #[test]
    fn a_manual_size_keeps_both_dimensions() {
        assert_eq!(
            processed_size(ImageSize::Manual {
                width: 64,
                height: 64
            }),
            (64, 64)
        );
    }

    /// Given an image with no requested size, when it is processed, then the source dimensions are kept.
    #[test]
    fn an_automatic_size_keeps_the_source_dimensions() {
        assert_eq!(processed_size(ImageSize::Automatic), (400, 200));
    }
}

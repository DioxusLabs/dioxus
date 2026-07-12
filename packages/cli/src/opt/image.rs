use anyhow::Context;
use image::{DynamicImage, EncodableLayout};
use manganis_core::{ImageAssetOptions, ImageFormat, ImageSize};
use std::io::{BufWriter, Write};
use std::path::Path;

pub(crate) fn process_image(
    image_options: &ImageAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    // Copy verbatim if no transform is requested (#5642)
    if image_options.format() == ImageFormat::Unknown
        && image_options.size() == ImageSize::Automatic
    {
        std::fs::copy(source, output_path).with_context(|| {
            format!(
                "Failed to copy image from {} to {}",
                source.display(),
                output_path.display()
            )
        })?;
        return Ok(());
    }

    let mut image = image::ImageReader::new(std::io::Cursor::new(&*std::fs::read(source)?))
        .with_guessed_format()
        .context("Failed to guess image format")?
        .decode();

    if let Ok(image) = &mut image {
        if let ImageSize::Manual { width, height } = image_options.size() {
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

    // Write a PNG with detectable chunk/compression to verify byte-for-byte copying
    fn write_source_png(path: &Path) {
        let file = std::fs::File::create(path).unwrap();
        let w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, 4, 4);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Best);
        encoder
            .add_text_chunk("Comment".to_string(), "dioxus-asset-test".to_string())
            .unwrap();
        let mut writer = encoder.write_header().unwrap();
        // 4x4 RGBA pixels with varied colors.
        let mut data = Vec::with_capacity(4 * 4 * 4);
        for i in 0..16u8 {
            data.extend_from_slice(&[i.wrapping_mul(16), 255 - i, i, 255]);
        }
        writer.write_image_data(&data).unwrap();
        writer.finish().unwrap();
    }

    // Test for #5642: `asset!()` must copy bytes verbatim
    #[test]
    fn default_options_preserve_source_bytes() {
        let dir = std::env::temp_dir().join(format!("dx-image-opt-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.png");
        let output = dir.join("output.png");
        write_source_png(&source);

        process_image(&ImageAssetOptions::default(), &source, &output).unwrap();

        assert_eq!(
            std::fs::read(&source).unwrap(),
            std::fs::read(&output).unwrap(),
            "asset!() with default options must copy the source bytes verbatim, not re-encode"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}

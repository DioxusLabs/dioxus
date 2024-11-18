use std::path::{Path, PathBuf};

use anyhow::Context;
use jpg::compress_jpg;
use manganis_core::{Asset, ImageAssetOptions, ImageSize, ImageType};
use png::compress_png;

mod jpg;
mod png;

pub(crate) fn process_image(
    image_options: &ImageAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    let mut image = image::ImageReader::new(std::io::Cursor::new(&*std::fs::read(source)?))
        .with_guessed_format()?
        .decode()?;

    if let ImageSize::Manual { width, height } = image_options.size() {
        image = image.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
    }

    match image_options.format() {
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
        ImageType::Unknown => {
            let source_file = std::fs::File::open(source)?;
            let mut reader = std::io::BufReader::new(source_file);
            let output_file = std::fs::File::create(output_path)?;
            let mut writer = std::io::BufWriter::new(output_file);
            std::io::copy(&mut reader, &mut writer).with_context(|| {
                format!(
                    "Failed to write image to output location: {}",
                    output_path.display()
                )
            })?;
        }
    }

    Ok(())
}

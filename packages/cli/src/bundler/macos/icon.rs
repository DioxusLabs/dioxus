use crate::bundler::BundleContext;
use anyhow::{Context, Result};
use image::DynamicImage;
use image::ImageReader;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

/// The icon sizes (in points) we generate for the .icns file, along with their
/// densities. macOS expects both 1x and 2x variants.
const ICON_SIZES: &[(u32, u32, u32)] = &[
    // (width, height, density)
    (16, 16, 1),
    (16, 16, 2),
    (32, 32, 1),
    (32, 32, 2),
    (64, 64, 1),
    (64, 64, 2),
    (128, 128, 1),
    (128, 128, 2),
    (256, 256, 1),
    (256, 256, 2),
    (512, 512, 1),
    (512, 512, 2),
];

/// Create an ICNS file from the icon files configured in the bundle context.
///
/// If the icon files already include an `.icns` file, it is copied directly.
/// If PNG files are provided, they are converted into an `.icns` file using
/// the `icns` crate.
///
/// Returns `Ok(Some(path))` with the path to the generated `.icns` file,
/// or `Ok(None)` if no icon files are configured.
pub(crate) fn create_icns_file(out_dir: &Path, ctx: &BundleContext) -> Result<Option<PathBuf>> {
    let icon_paths = ctx.icon_files()?;
    if icon_paths.is_empty() {
        return Ok(None);
    }

    let dest_path = out_dir.join(format!("{}.icns", ctx.product_name()));

    // If any of the icon files is already an .icns file, just copy it.
    for icon_path in &icon_paths {
        if icon_path
            .extension()
            .map(|ext| ext.eq_ignore_ascii_case("icns"))
            .unwrap_or(false)
        {
            tracing::info!("Copying existing .icns file: {}", icon_path.display());
            std::fs::copy(icon_path, &dest_path).with_context(|| {
                format!(
                    "Failed to copy .icns file from {} to {}",
                    icon_path.display(),
                    dest_path.display()
                )
            })?;
            return Ok(Some(dest_path));
        }
    }

    // Otherwise, build an ICNS from PNG images.
    let mut family = icns::IconFamily::new();

    for icon_path in &icon_paths {
        let ext = icon_path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if ext != "png" {
            tracing::debug!("Skipping non-PNG icon file: {}", icon_path.display());
            continue;
        }

        let img = load_png(icon_path)?;
        add_image_to_family(&mut family, &img)?;
    }

    if family.is_empty() {
        tracing::warn!("No valid icon images found; skipping .icns generation");
        return Ok(None);
    }

    let file = File::create(&dest_path)
        .with_context(|| format!("Failed to create {}", dest_path.display()))?;
    let writer = BufWriter::new(file);
    family
        .write(writer)
        .with_context(|| format!("Failed to write .icns to {}", dest_path.display()))?;

    tracing::info!("Generated .icns at {}", dest_path.display());
    Ok(Some(dest_path))
}

/// Load a PNG image from disk.
fn load_png(path: &Path) -> Result<DynamicImage> {
    let reader = ImageReader::open(path)
        .with_context(|| format!("Failed to open icon image: {}", path.display()))?;
    reader
        .decode()
        .with_context(|| format!("Failed to decode icon image: {}", path.display()))
}

/// Add all appropriate size variants of an image to the ICNS family.
///
/// The source image is resized to each target size and added with the
/// correct icon type for that size and density.
fn add_image_to_family(family: &mut icns::IconFamily, img: &DynamicImage) -> Result<()> {
    for &(width, height, density) in ICON_SIZES {
        let pixel_width = width * density;
        let pixel_height = height * density;

        let icon_type = match icns::IconType::from_pixel_size_and_density(
            pixel_width,
            pixel_height,
            density,
        ) {
            Some(t) => t,
            None => continue,
        };

        // Skip if we already have this icon type (e.g. from multiple source PNGs).
        if family.has_icon_with_type(icon_type) {
            continue;
        }

        let resized = img.resize_exact(
            pixel_width,
            pixel_height,
            image::imageops::FilterType::Lanczos3,
        );

        let rgba = resized.to_rgba8();
        let icns_image = icns::Image::from_data(
            icns::PixelFormat::RGBA,
            pixel_width,
            pixel_height,
            rgba.into_raw(),
        )
        .with_context(|| {
            format!("Failed to create icns::Image for {pixel_width}x{pixel_height}@{density}x")
        })?;

        family
            .add_icon_with_type(&icns_image, icon_type)
            .with_context(|| {
                format!("Failed to add icon type {icon_type:?} to ICNS family")
            })?;
    }

    Ok(())
}

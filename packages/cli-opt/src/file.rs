use anyhow::Context;
use manganis_core::{
    AssetOptions, CssAssetOptions, FolderAssetOptions, ImageAssetOptions, JsAssetOptions,
};
use std::path::Path;

use crate::css::process_scss;

use super::{
    css::process_css, folder::process_folder, image::process_image, js::process_js,
    json::process_json,
};

/// Process a specific file asset with the given options reading from the source and writing to the output path
pub fn process_file_to(
    options: &AssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    if check_output_path(output_path)? {
        return Ok(());
    }

    match options {
        AssetOptions::Unknown => match source.extension().map(|e| e.to_string_lossy()).as_deref() {
            Some("css") => {
                process_css(&CssAssetOptions::new(), source, output_path)?;
            }
            Some("scss" | "sass") => {
                process_scss(&CssAssetOptions::new(), source, output_path)?;
            }
            Some("js") => {
                process_js(&JsAssetOptions::new(), source, output_path)?;
            }
            Some("json") => {
                process_json(source, output_path)?;
            }
            Some("jpg" | "jpeg" | "png" | "webp" | "avif") => {
                process_image(&ImageAssetOptions::new(), source, output_path)?;
            }
            Some(_) | None => {
                match source.is_dir() {
                    true => process_folder(&FolderAssetOptions::new(), source, output_path)?,
                    false => copy_file_to(source, output_path)?,
                };
            }
        },
        AssetOptions::Css(options) => {
            process_css(options, source, output_path)?;
        }
        AssetOptions::Js(options) => {
            process_js(options, source, output_path)?;
        }
        AssetOptions::Image(options) => {
            process_image(options, source, output_path)?;
        }
        AssetOptions::Folder(options) => {
            process_folder(options, source, output_path)?;
        }
        _ => {
            tracing::warn!("Unknown asset options: {:?}", options);
        }
    }

    Ok(())
}

/// Copies an asset to it's destination without any processing.
pub fn copy_file_to(source: &Path, output_path: &Path) -> anyhow::Result<()> {
    if check_output_path(output_path)? {
        return Ok(());
    }

    let source_file = std::fs::File::open(source)?;
    let mut reader = std::io::BufReader::new(source_file);
    let output_file = std::fs::File::create(output_path)?;
    let mut writer = std::io::BufWriter::new(output_file);
    std::io::copy(&mut reader, &mut writer).with_context(|| {
        format!(
            "Failed to write file to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
}

/// Check the asset output path to ensure that:
/// 1. The asset doesn't already exist.
/// 2. That the parent path exists or is created.
///
/// Returns true if asset processing should be skipped.
fn check_output_path(output_path: &Path) -> anyhow::Result<bool> {
    // If the file already exists, then we must have a file with the same hash
    // already. The hash has the file contents and options, so if we find a file
    // with the same hash, we probably already created this file in the past
    if output_path.exists() {
        return Ok(true);
    }
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    Ok(false)
}

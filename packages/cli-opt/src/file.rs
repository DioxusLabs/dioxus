use anyhow::Context;
use manganis_core::{AssetOptions, CssAssetOptions, ImageAssetOptions, JsAssetOptions};
use std::{ffi::OsStr, path::Path};

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
    process_file_to_with_options(options, source, output_path, false)
}

/// Process a specific file asset with additional options
pub(crate) fn process_file_to_with_options(
    options: &AssetOptions,
    source: &Path,
    output_path: &Path,
    in_folder: bool,
) -> anyhow::Result<()> {
    // If the file already exists, then we must have a file with the same hash
    // already. The hash has the file contents and options, so if we find a file
    // with the same hash, we probably already created this file in the past
    if output_path.exists() {
        return Ok(());
    }
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // Processing can be slow. Write to a temporary file first and then rename it to the final output path. If everything
    // goes well. Without this, the user could quit in the middle of processing and the file will look complete to the
    // caching system even though it is empty.
    let mut partial_ext = output_path.extension().unwrap_or_default().to_os_string();
    partial_ext.push(OsStr::new(".partial"));
    let temp_output_path = output_path.with_extension(&partial_ext);
    let temp_output_path = &temp_output_path;

    match options {
        AssetOptions::Unknown => match source.extension().map(|e| e.to_string_lossy()).as_deref() {
            Some("css") => {
                process_css(&CssAssetOptions::new(), source, temp_output_path)?;
            }
            Some("scss" | "sass") => {
                process_scss(&CssAssetOptions::new(), source, temp_output_path)?;
            }
            Some("js") => {
                process_js(&JsAssetOptions::new(), source, temp_output_path, !in_folder)?;
            }
            Some("json") => {
                process_json(source, temp_output_path)?;
            }
            Some("jpg" | "jpeg" | "png" | "webp" | "avif") => {
                process_image(&ImageAssetOptions::new(), source, temp_output_path)?;
            }
            Some(_) | None => {
                if source.is_dir() {
                    process_folder(source, temp_output_path)?;
                } else {
                    let source_file = std::fs::File::open(source)?;
                    let mut reader = std::io::BufReader::new(source_file);
                    let output_file = std::fs::File::create(temp_output_path)?;
                    let mut writer = std::io::BufWriter::new(output_file);
                    std::io::copy(&mut reader, &mut writer).with_context(|| {
                        format!(
                            "Failed to write file to output location: {}",
                            temp_output_path.display()
                        )
                    })?;
                }
            }
        },
        AssetOptions::Css(options) => {
            process_css(options, source, temp_output_path)?;
        }
        AssetOptions::Js(options) => {
            process_js(options, source, temp_output_path, !in_folder)?;
        }
        AssetOptions::Image(options) => {
            process_image(options, source, temp_output_path)?;
        }
        AssetOptions::Folder(_) => {
            process_folder(source, temp_output_path)?;
        }
        _ => {
            tracing::warn!("Unknown asset options: {:?}", options);
        }
    }

    // If everything was successful, rename the temp file to the final output path
    std::fs::rename(temp_output_path, output_path)?;

    Ok(())
}

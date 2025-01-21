use anyhow::Context;
use manganis_core::{AssetOptions, CssAssetOptions, ImageAssetOptions, JsAssetOptions};
use std::path::Path;

use crate::css::{process_css_module, process_scss};

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
                if source.is_dir() {
                    process_folder(source, output_path)?;
                } else {
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
                }
            }
        },
        AssetOptions::Css(options) => {
            process_css(options, source, output_path)?;
        }
        AssetOptions::CssModule(options) => {
            process_css_module(options, source, output_path)?;
        }
        AssetOptions::Js(options) => {
            process_js(options, source, output_path)?;
        }
        AssetOptions::Image(options) => {
            process_image(options, source, output_path)?;
        }
        AssetOptions::Folder(_) => {
            process_folder(source, output_path)?;
        }
        _ => {
            tracing::warn!("Unknown asset options: {:?}", options);
        }
    }

    Ok(())
}

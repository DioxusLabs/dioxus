use anyhow::Context;
use manganis::{CssModuleAssetOptions, FolderAssetOptions};
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
            std::fs::create_dir_all(parent).context("Failed to create directory")?;
        }
    }

    // Processing can be slow. Write to a temporary file first and then rename it to the final output path. If everything
    // goes well. Without this, the user could quit in the middle of processing and the file will look complete to the
    // caching system even though it is empty.
    let temp_path = output_path.with_file_name(format!(
        "partial.{}",
        output_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    ));
    let resolved_options = resolve_asset_options(source, options);

    match &resolved_options {
        ResolvedAssetType::Css(options) => {
            process_css(options, source, &temp_path)?;
        }
        ResolvedAssetType::CssModule(options) => {
            process_css_module(options, source, output_path, &temp_path)?;
        }
        ResolvedAssetType::Scss(options) => {
            process_scss(options, source, &temp_path)?;
        }
        ResolvedAssetType::Js(options) => {
            process_js(options, source, &temp_path, !in_folder)?;
        }
        ResolvedAssetType::Image(options) => {
            process_image(options, source, &temp_path)?;
        }
        ResolvedAssetType::Json => {
            process_json(source, &temp_path)?;
        }
        ResolvedAssetType::Folder(_) => {
            process_folder(source, &temp_path)?;
        }
        ResolvedAssetType::File => {
            let source_file = std::fs::File::open(source)?;
            let mut reader = std::io::BufReader::new(source_file);
            let output_file = std::fs::File::create(&temp_path)?;
            let mut writer = std::io::BufWriter::new(output_file);
            std::io::copy(&mut reader, &mut writer).with_context(|| {
                format!(
                    "Failed to write file to output location: {}",
                    temp_path.display()
                )
            })?;
        }
    }

    // If everything was successful, rename the temp file to the final output path
    std::fs::rename(temp_path, output_path).context("Failed to rename output file")?;

    Ok(())
}

pub(crate) enum ResolvedAssetType {
    /// An image asset
    Image(ImageAssetOptions),
    /// A css asset
    Css(CssAssetOptions),
    /// A css module asset
    CssModule(CssModuleAssetOptions),
    /// A SCSS asset
    Scss(CssAssetOptions),
    /// A javascript asset
    Js(JsAssetOptions),
    /// A json asset
    Json,
    /// A folder asset
    Folder(FolderAssetOptions),
    /// A generic file
    File,
}

pub(crate) fn resolve_asset_options(source: &Path, options: &AssetOptions) -> ResolvedAssetType {
    match options {
        AssetOptions::Image(image) => ResolvedAssetType::Image(*image),
        AssetOptions::Css(css) => ResolvedAssetType::Css(*css),
        AssetOptions::CssModule(css) => ResolvedAssetType::CssModule(*css),
        AssetOptions::Js(js) => ResolvedAssetType::Js(*js),
        AssetOptions::Folder(folder) => ResolvedAssetType::Folder(*folder),
        AssetOptions::Unknown => resolve_unknown_asset_options(source),
        _ => {
            tracing::warn!("Unknown asset options... you may need to update the Dioxus CLI. Defaulting to a generic file: {:?}", options);
            resolve_unknown_asset_options(source)
        }
    }
}

fn resolve_unknown_asset_options(source: &Path) -> ResolvedAssetType {
    match source.extension().map(|e| e.to_string_lossy()).as_deref() {
        Some("scss" | "sass") => ResolvedAssetType::Scss(CssAssetOptions::new()),
        Some("css") => ResolvedAssetType::Css(CssAssetOptions::new()),
        Some("js") => ResolvedAssetType::Js(JsAssetOptions::new()),
        Some("json") => ResolvedAssetType::Json,
        Some("jpg" | "jpeg" | "png" | "webp" | "avif") => {
            ResolvedAssetType::Image(ImageAssetOptions::new())
        }
        _ if source.is_dir() => ResolvedAssetType::Folder(FolderAssetOptions::new()),
        _ => ResolvedAssetType::File,
    }
}

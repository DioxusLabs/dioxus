//! Freedesktop .desktop file generation and icon management for Linux.
//!
//! Handles creating XDG-compliant .desktop files and copying icons
//! into the proper hicolor icon theme directory hierarchy.

use crate::bundler::{category::AppCategory, BundleContext};
use anyhow::{Context, Result};
use handlebars::Handlebars;
use std::{
    fs,
    io::BufReader,
    path::{Path, PathBuf},
};

/// Default .desktop file template (Handlebars).
const DEFAULT_DESKTOP_TEMPLATE: &str = "[Desktop Entry]
Categories={{categories}}
{{#if comment}}
Comment={{comment}}
{{/if}}
Exec={{exec}}
Icon={{icon}}
Name={{name}}
Terminal=false
Type=Application
";

/// Generate the contents of a .desktop file for the given bundle context.
///
/// If `desktop_template` is provided, that file is used as a Handlebars template
/// instead of the built-in default. Available template variables:
/// `categories`, `comment` (optional), `exec`, `icon`, `name`.
pub(crate) fn generate_desktop_file(
    ctx: &BundleContext,
    desktop_template: Option<&Path>,
) -> Result<String> {
    let mut handlebars = Handlebars::new();
    // Do not use strict mode: the `comment` variable is optional and may not be present.
    handlebars.set_strict_mode(false);

    let template = if let Some(path) = desktop_template {
        fs::read_to_string(path)
            .with_context(|| format!("Failed to read desktop template: {}", path.display()))?
    } else {
        DEFAULT_DESKTOP_TEMPLATE.to_string()
    };

    handlebars
        .register_template_string("desktop", &template)
        .context("Failed to register desktop template")?;

    // Build the categories string from the app category setting.
    let categories = ctx
        .app_category()
        .and_then(|c| c.parse::<AppCategory>().ok())
        .map(|cat| cat.freedesktop_categories().to_string())
        .unwrap_or_default();

    let bin_name = ctx.main_binary_name();
    let product_name = ctx.product_name();

    // Comment is optional - use short_description if available.
    let description = ctx.short_description();
    let has_comment = !description.is_empty();

    // Use serde_json::Value so handlebars can handle the optional `comment` with {{#if}}.
    let mut json_data = serde_json::Map::new();
    json_data.insert("categories".into(), serde_json::Value::String(categories));
    json_data.insert(
        "exec".into(),
        serde_json::Value::String(bin_name.to_string()),
    );
    json_data.insert(
        "icon".into(),
        serde_json::Value::String(bin_name.to_string()),
    );
    json_data.insert("name".into(), serde_json::Value::String(product_name));
    if has_comment {
        json_data.insert("comment".into(), serde_json::Value::String(description));
    }

    let rendered = handlebars
        .render("desktop", &json_data)
        .context("Failed to render desktop template")?;

    Ok(rendered)
}

/// Copy icon files into the freedesktop hicolor icon theme hierarchy.
///
/// PNG icons are placed into `{data_dir}/usr/share/icons/hicolor/{size}x{size}/apps/{name}.png`
/// where `{size}` is determined by reading the PNG header.
///
/// Returns the list of paths that were written.
pub(crate) fn copy_icons(ctx: &BundleContext, data_dir: &Path) -> Result<Vec<PathBuf>> {
    let icon_files = ctx.icon_files()?;
    let bin_name = ctx.main_binary_name();
    let mut paths = Vec::new();

    for icon_path in &icon_files {
        let ext = icon_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "png" => {
                let file = fs::File::open(icon_path)
                    .with_context(|| format!("Failed to open PNG icon: {}", icon_path.display()))?;
                let decoder = png::Decoder::new(BufReader::new(file));
                let reader = decoder.read_info().with_context(|| {
                    format!("Failed to decode PNG dimensions: {}", icon_path.display())
                })?;
                let info = reader.info();
                let (width, height) = (info.width, info.height);

                // Use the larger dimension as the icon size (icons should be square, but
                // we handle non-square gracefully).
                let size = width.max(height);

                let dest_dir = data_dir.join(format!("usr/share/icons/hicolor/{size}x{size}/apps"));
                fs::create_dir_all(&dest_dir)?;

                let dest = dest_dir.join(format!("{bin_name}.png"));
                fs::copy(icon_path, &dest).with_context(|| {
                    format!(
                        "Failed to copy icon {} -> {}",
                        icon_path.display(),
                        dest.display()
                    )
                })?;

                tracing::debug!("Copied icon {}x{}: {}", size, size, dest.display());
                paths.push(dest);
            }
            "svg" => {
                // SVG icons go to the scalable directory.
                let dest_dir = data_dir.join("usr/share/icons/hicolor/scalable/apps");
                fs::create_dir_all(&dest_dir)?;

                let dest = dest_dir.join(format!("{bin_name}.svg"));
                fs::copy(icon_path, &dest).with_context(|| {
                    format!(
                        "Failed to copy icon {} -> {}",
                        icon_path.display(),
                        dest.display()
                    )
                })?;

                tracing::debug!("Copied SVG icon: {}", dest.display());
                paths.push(dest);
            }
            _ => {
                tracing::warn!(
                    "Skipping icon with unsupported extension '{}': {}",
                    ext,
                    icon_path.display()
                );
            }
        }
    }

    Ok(paths)
}

/// Find the path to the largest PNG icon from the icon files.
/// This is used for AppImage and other contexts that need a single "best" icon.
pub(crate) fn find_largest_icon(ctx: &BundleContext) -> Result<Option<PathBuf>> {
    let icon_files = ctx.icon_files()?;
    let mut best: Option<(u32, PathBuf)> = None;

    for icon_path in icon_files {
        let ext = icon_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "png" {
            if let Ok(file) = fs::File::open(&icon_path) {
                let decoder = png::Decoder::new(BufReader::new(file));
                if let Ok(reader) = decoder.read_info() {
                    let info = reader.info();
                    let (w, h) = (info.width, info.height);
                    let size = w.max(h);
                    if best
                        .as_ref()
                        .map_or(true, |(best_size, _)| size > *best_size)
                    {
                        best = Some((size, icon_path));
                    }
                }
            }
        }
    }

    Ok(best.map(|(_, path)| path))
}

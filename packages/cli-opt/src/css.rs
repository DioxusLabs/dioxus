use std::path::Path;

use anyhow::Context;
use codemap::SpanLoc;
use grass::OutputStyle;
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
};
use manganis_core::CssAssetOptions;
use tracing::{debug, warn};

pub(crate) fn process_css(
    css_options: &CssAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    let css = std::fs::read_to_string(source)?;

    let css = if css_options.minified() {
        minify_css(&css)
    } else {
        css
    };

    let updated_css = update_asset_references(&css);

    std::fs::write(output_path, updated_css).with_context(|| {
        format!(
            "Failed to write css to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
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

pub(crate) fn update_asset_references(css: &str) -> String {
    // Placeholder implementation for updating asset references in CSS files
    // This function should identify and update asset references to the new generated names
    css.to_string()
}

/// Process an scss/sass file into css.
pub(crate) fn process_scss(
    scss_options: &CssAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    let style = match scss_options.minified() {
        true => OutputStyle::Compressed,
        false => OutputStyle::Expanded,
    };

    let options = grass::Options::default()
        .style(style)
        .quiet(false)
        .logger(&ScssLogger {});

    let css = grass::from_path(source, &options)?;
    let minified = minify_css(&css);

    std::fs::write(output_path, minified).with_context(|| {
        format!(
            "Failed to write css to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
}

/// Logger for Grass that re-uses their StdLogger formatting but with tracing.
#[derive(Debug)]
struct ScssLogger {}

impl grass::Logger for ScssLogger {
    fn debug(&self, location: SpanLoc, message: &str) {
        debug!(
            "{}:{} DEBUG: {}",
            location.file.name(),
            location.begin.line + 1,
            message
        );
    }

    fn warn(&self, location: SpanLoc, message: &str) {
        warn!(
            "Warning: {}\n    ./{}:{}:{}",
            message,
            location.file.name(),
            location.begin.line + 1,
            location.begin.column + 1
        );
    }
}

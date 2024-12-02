use std::path::Path;

use anyhow::Context;
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
};
use manganis_core::CssAssetOptions;

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

    std::fs::write(output_path, css).with_context(|| {
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

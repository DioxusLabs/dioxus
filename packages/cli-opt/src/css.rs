use std::path::Path;

use anyhow::Context;
use codemap::SpanLoc;
use grass::OutputStyle;
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
    targets::{Browsers, Targets},
};
use manganis_core::CssAssetOptions;

pub(crate) fn process_css(
    css_options: &CssAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    let css = std::fs::read_to_string(source)?;

    let css = if css_options.minified() {
        // Try to minify the css. If we fail, log the error and use the unminified css
        match minify_css(&css) {
            Ok(minified) => minified,
            Err(err) => {
                tracing::error!(
                    "Failed to minify css; Falling back to unminified css. Error: {}",
                    err
                );
                css
            }
        }
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

pub(crate) fn minify_css(css: &str) -> anyhow::Result<String> {
    let options = ParserOptions {
        error_recovery: true,
        ..Default::default()
    };
    let mut stylesheet = StyleSheet::parse(css, options).map_err(|err| err.into_owned())?;

    // We load the browser list from the standard browser list file or use the browserslist default if we don't find any
    // settings. Without the browser lists default, lightningcss will default to supporting only the newest versions of
    // browsers.
    let browsers_list = match Browsers::load_browserslist()? {
        Some(browsers) => Some(browsers),
        None => {
            Browsers::from_browserslist(["defaults"]).expect("borwserslists should have defaults")
        }
    };

    let targets = Targets {
        browsers: browsers_list,
        ..Default::default()
    };

    stylesheet.minify(MinifyOptions {
        targets,
        ..Default::default()
    })?;
    let printer = PrinterOptions {
        targets,
        minify: true,
        ..Default::default()
    };
    let res = stylesheet.to_css(printer)?;
    Ok(res.code)
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
    let minified = minify_css(&css)?;

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
        tracing::debug!(
            "{}:{} DEBUG: {}",
            location.file.name(),
            location.begin.line + 1,
            message
        );
    }

    fn warn(&self, location: SpanLoc, message: &str) {
        tracing::warn!(
            "Warning: {}\n    ./{}:{}:{}",
            message,
            location.file.name(),
            location.begin.line + 1,
            location.begin.column + 1
        );
    }
}

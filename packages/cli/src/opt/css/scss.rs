//! SCSS/Sass compilation via grass.

use std::{hash::Hasher, path::Path};

use anyhow::Context;
use codemap::SpanLoc;
use grass::OutputStyle;
use manganis_core::CssAssetOptions;

use super::minify_css;

/// Compile scss with grass.
pub(crate) fn compile_scss(
    scss_options: &CssAssetOptions,
    source: &Path,
) -> anyhow::Result<String> {
    let style = match scss_options.minified() {
        true => OutputStyle::Compressed,
        false => OutputStyle::Expanded,
    };

    let options = grass::Options::default()
        .style(style)
        .quiet(false)
        .logger(&ScssLogger {});

    let css = grass::from_path(source, &options)
        .with_context(|| format!("Failed to compile scss file: {}", source.display()))?;
    Ok(css)
}

/// Process an scss/sass file into css.
pub(crate) fn process_scss(
    scss_options: &CssAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    let css = compile_scss(scss_options, source)?;
    let minified = minify_css(&css)?;

    std::fs::write(output_path, minified).with_context(|| {
        format!(
            "Failed to write css to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
}

/// Hash the inputs to the scss file.
pub(crate) fn hash_scss(
    scss_options: &CssAssetOptions,
    source: &Path,
    hasher: &mut impl Hasher,
) -> anyhow::Result<()> {
    // Grass doesn't expose the ast for us to traverse the imports in the file. Instead of parsing scss ourselves
    // we just hash the expanded version of the file for now
    let css = compile_scss(scss_options, source)?;
    hasher.write(css.as_bytes());
    Ok(())
}

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

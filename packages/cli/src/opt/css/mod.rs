//! CSS processing: parsing, minification, asset URL rewriting, and CSS modules.

mod references;
mod scss;

use std::path::Path;

use anyhow::{anyhow, Context};
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
    targets::{Browsers, Targets},
    visitor::Visit,
};
use manganis_core::{create_module_hash, transform_css, CssAssetOptions, CssModuleAssetOptions};

use super::AssetProcessor;

pub(crate) use references::discover_css_references;
pub(crate) use references::hash_css;
pub(crate) use scss::{hash_scss, process_scss};

use references::AssetUrlRewriter;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn parse_stylesheet(css: &str) -> anyhow::Result<StyleSheet<'_, '_>> {
    StyleSheet::parse(
        css,
        ParserOptions {
            error_recovery: true,
            ..Default::default()
        },
    )
    .map_err(|err| err.into_owned().into())
}

fn browser_targets() -> anyhow::Result<Targets> {
    let browsers_list = match Browsers::load_browserslist()? {
        Some(browsers) => Some(browsers),
        None => {
            Browsers::from_browserslist(["defaults"]).expect("browserslist should have defaults")
        }
    };
    Ok(Targets {
        browsers: browsers_list,
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// CSS processing
// ---------------------------------------------------------------------------

impl AssetProcessor<'_> {
    fn css_rewriter<'a>(&'a self, css_dir: &'a Path) -> AssetUrlRewriter<'a> {
        AssetUrlRewriter {
            css_dir,
            manifest: self.manifest,
        }
    }

    /// Process a CSS file: rewrite asset references, optionally minify, then write the result.
    /// Single parse, single serialize.
    pub(crate) fn process_css(
        &self,
        css_options: &CssAssetOptions,
        source: &Path,
        output_path: &Path,
    ) -> anyhow::Result<()> {
        let css = std::fs::read_to_string(source)?;
        let css_dir = source.parent().unwrap_or(Path::new("."));
        let mut stylesheet = parse_stylesheet(&css)?;

        let mut rewriter = self.css_rewriter(css_dir);
        stylesheet.visit(&mut rewriter).unwrap();

        let printer = if css_options.minified() {
            let targets = browser_targets().unwrap_or_default();
            if let Err(err) = stylesheet.minify(MinifyOptions {
                targets,
                ..Default::default()
            }) {
                tracing::error!("Failed to minify css; falling back to unminified: {err}");
            }
            PrinterOptions {
                targets,
                minify: true,
                ..Default::default()
            }
        } else {
            PrinterOptions::default()
        };

        let result = stylesheet.to_css(printer)?;

        std::fs::write(output_path, result.code).with_context(|| {
            format!(
                "Failed to write css to output location: {}",
                output_path.display()
            )
        })
    }
}

pub(crate) fn process_css_module(
    css_options: &CssModuleAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    let css = std::fs::read_to_string(source)?;

    let mut src_name = source
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| {
            anyhow!(
                "Failed to read name of css module file `{}`.",
                source.display()
            )
        })?
        .strip_suffix(".css")
        .ok_or_else(|| {
            anyhow!(
                "Css module file `{}` should end with a `.css` suffix.",
                source.display(),
            )
        })?
        .to_string();

    src_name.push('-');

    let hash = create_module_hash(source);
    let css = transform_css(css.as_str(), hash.as_str()).map_err(|error| {
        anyhow!(
            "Invalid css for file `{}`\nError:\n{}",
            source.display(),
            error
        )
    })?;

    let css = if css_options.minified() {
        match minify_css(&css) {
            Ok(minified) => minified,
            Err(err) => {
                tracing::error!(
                    "Failed to minify css module; Falling back to unminified css. Error: {}",
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
            "Failed to write css module to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
}

pub(crate) fn minify_css(css: &str) -> anyhow::Result<String> {
    let mut stylesheet = parse_stylesheet(css)?;
    let targets = browser_targets()?;

    stylesheet.minify(MinifyOptions {
        targets,
        ..Default::default()
    })?;
    let res = stylesheet.to_css(PrinterOptions {
        targets,
        minify: true,
        ..Default::default()
    })?;
    Ok(res.code)
}

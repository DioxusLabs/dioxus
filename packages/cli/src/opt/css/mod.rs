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
pub(crate) use scss::hash_scss;

use references::AssetUrlRewriter;

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

impl AssetProcessor<'_> {
    fn css_rewriter<'a>(&'a self, css_dir: &'a Path) -> AssetUrlRewriter<'a> {
        AssetUrlRewriter {
            css_dir,
            manifest: self.manifest,
            public_asset_root: &self.public_asset_root,
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
        // Error type is Infallible — cannot fail
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

    /// Process a CSS module: apply module scoping, rewrite asset references, optionally minify.
    pub(crate) fn process_css_module(
        &self,
        css_options: &CssModuleAssetOptions,
        source: &Path,
        output_path: &Path,
    ) -> anyhow::Result<()> {
        let css = std::fs::read_to_string(source)?;
        let css_dir = source.parent().unwrap_or(Path::new("."));

        let hash = create_module_hash(source);
        let css = transform_css(css.as_str(), hash.as_str()).map_err(|error| {
            anyhow!(
                "Invalid css for file `{}`\nError:\n{}",
                source.display(),
                error
            )
        })?;

        let mut stylesheet = parse_stylesheet(&css)?;

        let mut rewriter = self.css_rewriter(css_dir);
        // Error type is Infallible — cannot fail
        stylesheet.visit(&mut rewriter).unwrap();

        let printer = if css_options.minified() {
            let targets = browser_targets().unwrap_or_default();
            if let Err(err) = stylesheet.minify(MinifyOptions {
                targets,
                ..Default::default()
            }) {
                tracing::error!("Failed to minify css module; falling back to unminified: {err}");
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
                "Failed to write css module to output location: {}",
                output_path.display()
            )
        })
    }

    /// Process an SCSS/Sass file: compile to CSS, rewrite asset references, minify, then write.
    pub(crate) fn process_scss(
        &self,
        scss_options: &CssAssetOptions,
        source: &Path,
        output_path: &Path,
    ) -> anyhow::Result<()> {
        let css = scss::compile_scss(scss_options, source)?;
        let css_dir = source.parent().unwrap_or(Path::new("."));
        let mut stylesheet = parse_stylesheet(&css)?;

        let mut rewriter = self.css_rewriter(css_dir);
        // Error type is Infallible — cannot fail
        stylesheet.visit(&mut rewriter).unwrap();

        let targets = browser_targets().unwrap_or_default();
        if let Err(err) = stylesheet.minify(MinifyOptions {
            targets,
            ..Default::default()
        }) {
            tracing::error!("Failed to minify scss output; falling back to unminified: {err}");
        }
        let result = stylesheet.to_css(PrinterOptions {
            targets,
            minify: true,
            ..Default::default()
        })?;

        std::fs::write(output_path, result.code).with_context(|| {
            format!(
                "Failed to write css to output location: {}",
                output_path.display()
            )
        })
    }
}

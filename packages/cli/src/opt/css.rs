use std::{
    collections::{HashSet, VecDeque},
    hash::Hasher,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use codemap::SpanLoc;
use grass::OutputStyle;
use lightningcss::{
    dependencies::{Dependency, DependencyOptions},
    printer::PrinterOptions,
    rules::CssRule,
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
    targets::{Browsers, Targets},
    values::url::Url,
    visit_types,
    visitor::{Visit, VisitTypes, Visitor},
};
use manganis_core::{create_module_hash, transform_css, CssAssetOptions, CssModuleAssetOptions};

use super::AssetManifest;

/// Returns true if the URL is external and should not be rewritten.
fn is_external_url(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("data:")
        || url.starts_with('#')
        || url.starts_with("blob:")
}

/// Resolve a URL to an absolute path on disk, if the file exists.
fn resolve_css_url(url: &str, css_dir: &Path) -> Option<PathBuf> {
    if is_external_url(url) {
        return None;
    }
    let resolved = css_dir.join(url);
    dunce::canonicalize(&resolved).ok().filter(|p| p.exists())
}

/// Parse a CSS file and return the resolved local paths of all `url()` and `@import` references.
fn extract_css_dep_paths(css: &str, css_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let options = ParserOptions {
        error_recovery: true,
        ..Default::default()
    };
    let stylesheet = StyleSheet::parse(css, options).map_err(|err| err.into_owned())?;

    let printer = PrinterOptions {
        analyze_dependencies: Some(DependencyOptions {
            remove_imports: false,
        }),
        ..Default::default()
    };
    let result = stylesheet.to_css(printer)?;

    Ok(result
        .dependencies
        .unwrap_or_default()
        .into_iter()
        .filter_map(|dep| {
            let url = match dep {
                Dependency::Import(i) => i.url,
                Dependency::Url(u) => u.url,
            };
            resolve_css_url(&url, css_dir)
        })
        .collect())
}

/// Collect the resolved paths of all local assets referenced by a CSS file.
///
/// Recursively follows `@import` to discover transitive dependencies.
fn collect_css_referenced_paths(source: &Path, visited: &mut HashSet<PathBuf>) -> Vec<PathBuf> {
    let canonical = dunce::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    if !visited.insert(canonical) {
        return vec![];
    }

    let css = match std::fs::read_to_string(source) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let css_dir = source.parent().unwrap_or(Path::new("."));

    let dep_paths = match extract_css_dep_paths(&css, css_dir) {
        Ok(p) => p,
        Err(_) => return vec![],
    };

    let mut paths = Vec::new();
    for ref_path in dep_paths {
        paths.push(ref_path.clone());
        // Recurse into imported CSS files
        if ref_path.extension().is_some_and(|e| e == "css") {
            paths.extend(collect_css_referenced_paths(&ref_path, visited));
        }
    }
    paths
}

// ---------------------------------------------------------------------------
// Public API: discovery, hashing, processing
// ---------------------------------------------------------------------------

/// Discover assets referenced by CSS files in the manifest and register them.
///
/// Walks all CSS assets currently in `manifest`, parses them for `url()` and
/// `@import` references, and registers any newly-discovered files as assets.
/// Newly-discovered CSS files are themselves scanned (breadth-first).
pub(crate) fn discover_css_references(manifest: &mut AssetManifest) -> anyhow::Result<()> {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut queue: VecDeque<PathBuf> = manifest.css_source_paths().into();

    while let Some(css_source) = queue.pop_front() {
        let canonical = dunce::canonicalize(&css_source).unwrap_or_else(|_| css_source.clone());
        if !visited.insert(canonical.clone()) {
            continue;
        }

        let css = match std::fs::read_to_string(&canonical) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!("Failed to read CSS {}: {e}", canonical.display());
                continue;
            }
        };
        let css_dir = canonical.parent().unwrap_or(Path::new("."));

        let dep_paths = match extract_css_dep_paths(&css, css_dir) {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!(
                    "Failed to parse CSS references from {}: {e}",
                    canonical.display()
                );
                continue;
            }
        };

        for ref_path in &dep_paths {

            if manifest.get_first_asset_for_source(ref_path).is_some() {
                continue;
            }

            let options = infer_asset_options(ref_path);
            match manifest.register_asset(ref_path, options) {
                Ok(_) => {
                    tracing::debug!("Registered CSS-referenced asset: {}", ref_path.display());
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to register CSS-referenced asset {}: {e}",
                        ref_path.display()
                    );
                    continue;
                }
            }

            if ref_path
                .extension()
                .is_some_and(|ext| ext == "css" || ext == "scss" || ext == "sass")
            {
                queue.push_back(ref_path.clone());
            }
        }
    }

    Ok(())
}

/// Hash a CSS file's contents together with all assets it references.
///
/// This ensures the CSS content hash changes when any referenced asset changes,
/// so the cached output is invalidated.
pub(super) fn hash_css(
    source: &Path,
    hasher: &mut impl Hasher,
) -> anyhow::Result<()> {
    super::hash::hash_file_contents(source, hasher)?;

    let mut visited = HashSet::new();
    let mut ref_paths = collect_css_referenced_paths(source, &mut visited);
    ref_paths.sort();
    ref_paths.dedup();

    for ref_path in ref_paths {
        if let Err(e) = super::hash::hash_file_contents(&ref_path, hasher) {
            tracing::debug!(
                "Failed to hash CSS-referenced file {}: {e}",
                ref_path.display()
            );
        }
    }

    Ok(())
}

/// Process a CSS file, optionally rewriting asset references to hashed paths.
pub(crate) fn process_css(
    css_options: &CssAssetOptions,
    source: &Path,
    output_path: &Path,
    manifest: Option<&AssetManifest>,
) -> anyhow::Result<()> {
    let css = std::fs::read_to_string(source)?;

    let css = match manifest {
        Some(manifest) => rewrite_css_urls(&css, source, manifest)?,
        None => css,
    };

    let css = maybe_minify(css_options, css);

    std::fs::write(output_path, css).with_context(|| {
        format!(
            "Failed to write css to output location: {}",
            output_path.display()
        )
    })
}

/// Resolve a URL string to its bundled asset path, if it exists in the manifest.
fn resolve_url_to_bundled_path(url: &str, css_dir: &Path, manifest: &AssetManifest) -> Option<String> {
    if is_external_url(url) {
        return None;
    }
    let resolved = css_dir.join(url);
    let canonical = dunce::canonicalize(&resolved).ok()?;
    let asset = manifest.get_first_asset_for_source(&canonical)?;
    Some(format!("/assets/{}", asset.bundled_path()))
}

/// Visitor that rewrites `url()` and `@import` references to hashed bundled paths.
struct AssetUrlRewriter<'a> {
    css_dir: &'a Path,
    manifest: &'a AssetManifest,
}

impl<'i> Visitor<'i> for AssetUrlRewriter<'_> {
    type Error = std::convert::Infallible;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(URLS | RULES)
    }

    fn visit_url(&mut self, url: &mut Url<'i>) -> Result<(), Self::Error> {
        if let Some(bundled) = resolve_url_to_bundled_path(&url.url, self.css_dir, self.manifest) {
            url.url = bundled.into();
        }
        Ok(())
    }

    fn visit_rule(&mut self, rule: &mut CssRule<'i>) -> Result<(), Self::Error> {
        // @import url field has #[skip_visit], so visit_url won't see it.
        // Handle it here by matching on the Import variant directly.
        if let CssRule::Import(import) = rule {
            if let Some(bundled) = resolve_url_to_bundled_path(&import.url, self.css_dir, self.manifest) {
                import.url = bundled.into();
            }
        }
        rule.visit_children(self)
    }
}

/// Rewrite `url()` and `@import` references in CSS to their hashed bundled paths.
fn rewrite_css_urls(css: &str, source: &Path, manifest: &AssetManifest) -> anyhow::Result<String> {
    let css_dir = source.parent().unwrap_or(Path::new("."));
    let options = ParserOptions {
        error_recovery: true,
        ..Default::default()
    };
    let mut stylesheet = StyleSheet::parse(css, options).map_err(|err| err.into_owned())?;

    let mut rewriter = AssetUrlRewriter { css_dir, manifest };
    stylesheet.visit(&mut rewriter).unwrap();

    let result = stylesheet.to_css(PrinterOptions::default())?;
    Ok(result.code)
}

// ---------------------------------------------------------------------------
// CSS module, SCSS, minification (unchanged)
// ---------------------------------------------------------------------------

pub(crate) fn process_css_module(
    css_options: &CssModuleAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    let css = std::fs::read_to_string(source)?;

    // Collect the file hash name.
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

    // Minify CSS
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

fn maybe_minify(css_options: &CssAssetOptions, css: String) -> String {
    if css_options.minified() {
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
    }
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

/// Compile scss with grass
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

/// Hash the inputs to the scss file
pub(crate) fn hash_scss(
    scss_options: &CssAssetOptions,
    source: &Path,
    hasher: &mut impl Hasher,
) -> anyhow::Result<()> {
    // Grass doesn't expose the ast for us to traverse the imports in the file. Instead of parsing scss ourselves
    // we just hash the expanded version of the file for now
    let css = compile_scss(scss_options, source)?;

    // Hash the compiled css
    hasher.write(css.as_bytes());

    Ok(())
}

/// Infer default asset options from a file's extension.
fn infer_asset_options(source: &Path) -> manganis::AssetOptions {
    match source.extension().map(|e| e.to_string_lossy()).as_deref() {
        Some("css") => manganis::AssetOptions::css().into_asset_options(),
        Some("js") => manganis::AssetOptions::js().into_asset_options(),
        _ => manganis::AssetOptions::builder().into_asset_options(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn extract_resolves_local_deps_and_skips_unresolvable() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "logo.png", "fake-png");
        write_file(dir.path(), "other.css", "body { margin: 0 }");

        let css = r#"
            @import "other.css";
            .a { background: url("logo.png"); }
            .b { background: url("https://example.com/img.png"); }
            .c { background: url("missing.png"); }
        "#;

        let paths = extract_css_dep_paths(css, dir.path()).unwrap();

        // Only local existing files are returned
        assert_eq!(paths.len(), 2);
        let names: HashSet<_> = paths
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .collect();
        assert!(names.contains("other.css"));
        assert!(names.contains("logo.png"));
    }

    #[test]
    fn collect_referenced_paths_recurses_imports() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "icon.png", "fake-icon");
        write_file(
            dir.path(),
            "base.css",
            r#".icon { background: url("icon.png"); }"#,
        );
        let entry = write_file(
            dir.path(),
            "main.css",
            r#"@import "base.css"; .app { color: red; }"#,
        );

        let mut visited = HashSet::new();
        let paths = collect_css_referenced_paths(&entry, &mut visited);

        // Should find both base.css and icon.png
        let names: HashSet<_> = paths
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .collect();
        assert!(names.contains("base.css"), "should find base.css");
        assert!(names.contains("icon.png"), "should find icon.png");
    }

    #[test]
    fn collect_handles_circular_imports() {
        let dir = TempDir::new().unwrap();
        // a.css imports b.css, b.css imports a.css
        write_file(dir.path(), "a.css", r#"@import "b.css"; .a {}"#);
        write_file(dir.path(), "b.css", r#"@import "a.css"; .b {}"#);

        let a = dir.path().join("a.css");
        let mut visited = HashSet::new();
        // Should terminate without infinite loop
        let paths = collect_css_referenced_paths(&a, &mut visited);
        assert!(!paths.is_empty());
    }

    #[test]
    fn rewrite_replaces_urls_with_bundled_paths() {
        let dir = TempDir::new().unwrap();
        let img_path = write_file(dir.path(), "logo.png", "fake-png");

        let css = r#".hero { background: url("logo.png"); }"#;

        let mut manifest = AssetManifest::default();
        manifest
            .register_asset(
                &img_path,
                manganis::AssetOptions::builder().into_asset_options(),
            )
            .unwrap();

        let css_path = dir.path().join("style.css");
        let result = rewrite_css_urls(css, &css_path, &manifest).unwrap();

        // Should contain the hashed path, not the original
        assert!(!result.contains("logo.png"), "original URL should be gone");
        assert!(
            result.contains("/assets/"),
            "should contain bundled asset path"
        );
    }

    #[test]
    fn rewrite_preserves_unknown_urls() {
        let dir = TempDir::new().unwrap();
        let css = r#".a { background: url("missing.png"); }"#;

        let manifest = AssetManifest::default();
        let css_path = dir.path().join("style.css");
        let result = rewrite_css_urls(css, &css_path, &manifest).unwrap();

        // Should fall back to original URL
        assert!(
            result.contains("missing.png"),
            "should keep original URL for unknown files"
        );
    }

    #[test]
    fn discover_registers_referenced_assets() {
        let dir = TempDir::new().unwrap();
        let img_path = write_file(dir.path(), "bg.png", "fake-png");
        let css_path = write_file(
            dir.path(),
            "style.css",
            r#".hero { background: url("bg.png"); }"#,
        );

        let mut manifest = AssetManifest::default();
        manifest
            .register_asset(
                &css_path,
                manganis::AssetOptions::css().into_asset_options(),
            )
            .unwrap();

        // bg.png is not yet in the manifest
        let canonical_img = dunce::canonicalize(&img_path).unwrap();
        assert!(manifest.get_first_asset_for_source(&canonical_img).is_none());

        discover_css_references(&mut manifest).unwrap();

        // Now it should be registered
        assert!(manifest.get_first_asset_for_source(&canonical_img).is_some());
    }

    #[test]
    fn hash_changes_when_referenced_asset_changes() {
        use std::collections::hash_map::DefaultHasher;

        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "img.png", "version-1");
        let css_path = write_file(
            dir.path(),
            "style.css",
            r#".a { background: url("img.png"); }"#,
        );

        let hash1 = {
            let mut h = DefaultHasher::new();
            hash_css(&css_path, &mut h).unwrap();
            h.finish()
        };

        // Change the referenced image
        write_file(dir.path(), "img.png", "version-2");

        let hash2 = {
            let mut h = DefaultHasher::new();
            hash_css(&css_path, &mut h).unwrap();
            h.finish()
        };

        assert_ne!(hash1, hash2, "CSS hash should change when referenced asset changes");
    }
}

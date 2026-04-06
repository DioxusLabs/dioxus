//! Discovery, hashing, and rewriting of asset references in CSS files.

use std::{
    collections::{HashSet, VecDeque},
    hash::Hasher,
    path::{Path, PathBuf},
};

use lightningcss::{
    rules::CssRule,
    values::url::Url,
    visit_types,
    visitor::{Visit, VisitTypes, Visitor},
};

use crate::opt::AssetManifest;

use super::parse_stylesheet;

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

// ---------------------------------------------------------------------------
// Visitor: collect resolved dependency paths (read-only)
// ---------------------------------------------------------------------------

struct UrlCollector<'a> {
    css_dir: &'a Path,
    paths: Vec<PathBuf>,
}

impl<'i> Visitor<'i> for UrlCollector<'_> {
    type Error = std::convert::Infallible;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(URLS | RULES)
    }

    fn visit_url(&mut self, url: &mut Url<'i>) -> Result<(), Self::Error> {
        if let Some(path) = resolve_css_url(&url.url, self.css_dir) {
            self.paths.push(path);
        }
        Ok(())
    }

    fn visit_rule(&mut self, rule: &mut CssRule<'i>) -> Result<(), Self::Error> {
        if let CssRule::Import(import) = rule {
            if let Some(path) = resolve_css_url(&import.url, self.css_dir) {
                self.paths.push(path);
            }
        }
        rule.visit_children(self)
    }
}

/// Parse CSS and return the resolved local paths of all `url()` and `@import` references.
fn extract_css_dep_paths(css: &str, css_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut stylesheet = parse_stylesheet(css)?;
    let mut collector = UrlCollector {
        css_dir,
        paths: Vec::new(),
    };
    stylesheet.visit(&mut collector).unwrap();
    Ok(collector.paths)
}

// ---------------------------------------------------------------------------
// Visitor: rewrite URLs to hashed bundled paths
// ---------------------------------------------------------------------------

pub(super) struct AssetUrlRewriter<'a> {
    pub css_dir: &'a Path,
    pub manifest: &'a AssetManifest,
}

impl AssetUrlRewriter<'_> {
    fn rewrite(&self, url: &str) -> Option<String> {
        if is_external_url(url) {
            return None;
        }
        let resolved = self.css_dir.join(url);
        let canonical = dunce::canonicalize(&resolved).ok()?;
        let asset = self.manifest.get_first_asset_for_source(&canonical)?;
        Some(format!("/assets/{}", asset.bundled_path()))
    }
}

impl<'i> Visitor<'i> for AssetUrlRewriter<'_> {
    type Error = std::convert::Infallible;

    fn visit_types(&self) -> VisitTypes {
        visit_types!(URLS | RULES)
    }

    fn visit_url(&mut self, url: &mut Url<'i>) -> Result<(), Self::Error> {
        if let Some(bundled) = self.rewrite(&url.url) {
            url.url = bundled.into();
        }
        Ok(())
    }

    fn visit_rule(&mut self, rule: &mut CssRule<'i>) -> Result<(), Self::Error> {
        if let CssRule::Import(import) = rule {
            if let Some(bundled) = self.rewrite(&import.url) {
                import.url = bundled.into();
            }
        }
        rule.visit_children(self)
    }
}

// ---------------------------------------------------------------------------
// Recursive path collection (for hashing)
// ---------------------------------------------------------------------------

/// Collect the resolved paths of all local assets referenced by a CSS file.
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
        if ref_path.extension().is_some_and(|e| e == "css") {
            paths.extend(collect_css_referenced_paths(&ref_path, visited));
        }
    }
    paths
}

// ---------------------------------------------------------------------------
// Public API
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
pub(crate) fn hash_css(source: &Path, hasher: &mut impl Hasher) -> anyhow::Result<()> {
    crate::opt::hash::hash_file_contents(source, hasher)?;

    let mut visited = HashSet::new();
    let mut ref_paths = collect_css_referenced_paths(source, &mut visited);
    ref_paths.sort();
    ref_paths.dedup();

    for ref_path in ref_paths {
        if let Err(e) = crate::opt::hash::hash_file_contents(&ref_path, hasher) {
            tracing::debug!(
                "Failed to hash CSS-referenced file {}: {e}",
                ref_path.display()
            );
        }
    }

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
    use crate::opt::css::process_css;
    use manganis_core::CssAssetOptions;
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

        let names: HashSet<_> = paths
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .collect();
        assert!(names.contains("base.css"));
        assert!(names.contains("icon.png"));
    }

    #[test]
    fn collect_handles_circular_imports() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "a.css", r#"@import "b.css"; .a {}"#);
        write_file(dir.path(), "b.css", r#"@import "a.css"; .b {}"#);

        let a = dir.path().join("a.css");
        let mut visited = HashSet::new();
        let paths = collect_css_referenced_paths(&a, &mut visited);
        assert!(!paths.is_empty());
    }

    #[test]
    fn process_rewrites_urls_and_minifies() {
        let dir = TempDir::new().unwrap();
        let img_path = write_file(dir.path(), "logo.png", "fake-png");

        let css_path = write_file(
            dir.path(),
            "style.css",
            r#".hero { background: url("logo.png"); color: red; }"#,
        );
        let output_path = dir.path().join("out.css");

        let mut manifest = AssetManifest::default();
        manifest
            .register_asset(
                &img_path,
                manganis::AssetOptions::builder().into_asset_options(),
            )
            .unwrap();

        let opts = CssAssetOptions::default();
        process_css(&opts, &css_path, &output_path, &manifest).unwrap();

        let result = std::fs::read_to_string(&output_path).unwrap();
        assert!(!result.contains("logo.png"), "original URL should be gone");
        assert!(result.contains("/assets/"));
        assert!(!result.contains("  "), "should be minified");
    }

    #[test]
    fn process_preserves_unknown_urls() {
        let dir = TempDir::new().unwrap();
        let css_path = write_file(
            dir.path(),
            "style.css",
            r#".a { background: url("missing.png"); }"#,
        );
        let output_path = dir.path().join("out.css");

        let manifest = AssetManifest::default();
        process_css(
            &CssAssetOptions::default(),
            &css_path,
            &output_path,
            &manifest,
        )
        .unwrap();

        let result = std::fs::read_to_string(&output_path).unwrap();
        assert!(result.contains("missing.png"));
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

        let canonical_img = dunce::canonicalize(&img_path).unwrap();
        assert!(manifest
            .get_first_asset_for_source(&canonical_img)
            .is_none());

        discover_css_references(&mut manifest).unwrap();

        assert!(manifest
            .get_first_asset_for_source(&canonical_img)
            .is_some());
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

        write_file(dir.path(), "img.png", "version-2");

        let hash2 = {
            let mut h = DefaultHasher::new();
            hash_css(&css_path, &mut h).unwrap();
            h.finish()
        };

        assert_ne!(
            hash1, hash2,
            "CSS hash should change when referenced asset changes"
        );
    }
}

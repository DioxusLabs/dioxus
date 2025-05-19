use crate::AssetOptions;
use const_serialize::SerializeConst;
use std::collections::HashSet;

/// Options for a css asset
#[derive(
    Debug,
    Eq,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CssAssetOptions {
    minify: bool,
    preload: bool,
}

impl Default for CssAssetOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl CssAssetOptions {
    /// Create a new css asset using the builder
    pub const fn new() -> Self {
        Self {
            preload: false,
            minify: true,
        }
    }

    /// Sets whether the css should be minified (default: true)
    ///
    /// Minifying the css can make your site load faster by loading less data
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, CssAssetOptions};
    /// const _: Asset = asset!("/assets/style.css", CssAssetOptions::new().with_minify(false));
    /// ```
    #[allow(unused)]
    pub const fn with_minify(self, minify: bool) -> Self {
        Self { minify, ..self }
    }

    /// Check if the asset is minified
    pub const fn minified(&self) -> bool {
        self.minify
    }

    /// Make the asset preloaded
    ///
    /// Preloading css will make the image start to load as soon as possible. This is useful for css that is used soon after the page loads or css that may not be used immediately, but should start loading sooner
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, CssAssetOptions};
    /// const _: Asset = asset!("/assets/style.css", CssAssetOptions::new().with_preload(true));
    /// ```
    #[allow(unused)]
    pub const fn with_preload(self, preload: bool) -> Self {
        Self { preload, ..self }
    }

    /// Check if the asset is preloaded
    pub const fn preloaded(&self) -> bool {
        self.preload
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions::Css(self)
    }
}

/// Options for a css module asset
#[derive(
    Debug,
    Eq,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[non_exhaustive]
pub struct CssModuleAssetOptions {
    minify: bool,
    preload: bool,
}

impl Default for CssModuleAssetOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl CssModuleAssetOptions {
    /// Create a new css asset using the builder
    pub const fn new() -> Self {
        Self {
            preload: false,
            minify: true,
        }
    }

    /// Sets whether the css should be minified (default: true)
    ///
    /// Minifying the css can make your site load faster by loading less data
    ///
    /// ```rust
    /// # use manganis::{styles, CssModuleAssetOptions};
    /// styles!(STYLES, "/assets/style.css", CssModuleAssetOptions::new().with_minify(false));
    /// ```
    #[allow(unused)]
    pub const fn with_minify(self, minify: bool) -> Self {
        Self { minify, ..self }
    }

    /// Check if the asset is minified
    pub const fn minified(&self) -> bool {
        self.minify
    }

    /// Make the asset preloaded
    ///
    /// Preloading css will make the image start to load as soon as possible. This is useful for css that is used soon after the page loads or css that may not be used immediately, but should start loading sooner
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, CssAssetOptions};
    /// styles!("/assets/style.css", CssAssetOptions::new().with_preload(true));
    /// ```
    #[allow(unused)]
    pub const fn with_preload(self, preload: bool) -> Self {
        Self { preload, ..self }
    }

    /// Check if the asset is preloaded
    pub const fn preloaded(&self) -> bool {
        self.preload
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions::CssModule(self)
    }
}

/// Collect CSS classes & ids.
///
/// This is a rudementary css classes & ids collector.
/// Idents used only in media queries will not be collected. (not support yet)
///
/// There are likely a number of edge cases that will show up.
///
/// Returns `(HashSet<Classes>, HashSet<Ids>)`
pub fn collect_css_idents(css: &str) -> (HashSet<String>, HashSet<String>) {
    const ALLOWED: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";

    let mut classes = HashSet::new();
    let mut ids = HashSet::new();

    // Collected ident name and true for ids.
    let mut start: Option<(String, bool)> = None;

    // True if we have the first comment start delimiter `/`
    let mut comment_start = false;
    // True if we have the first comment end delimiter '*'
    let mut comment_end = false;
    // True if we're in a comment scope.
    let mut in_comment_scope = false;

    // True if we're in a block scope: `#hi { this is block scope }`
    let mut in_block_scope = false;

    // If we are currently collecting an ident:
    // - Check if the char is allowed, put it into the ident string.
    // - If not allowed, finalize the ident string and reset start.
    // Otherwise:
    // Check if character is a `.` or `#` representing a class or string, and start collecting.
    for (_byte_index, c) in css.char_indices() {
        if let Some(ident) = start.as_mut() {
            if ALLOWED.find(c).is_some() {
                // CSS ignore idents that start with a number.
                // 1. Difficult to process
                // 2. Avoid false positives (transition: 0.5s)
                if ident.0.is_empty() && c.is_numeric() {
                    start = None;
                    continue;
                }

                ident.0.push(c);
            } else {
                match ident.1 {
                    true => ids.insert(ident.0.clone()),
                    false => classes.insert(ident.0.clone()),
                };

                start = None;
            }
        } else {
            // Handle entering an exiting scopede.
            match c {
                // Mark as comment scope if we have comment start: /*
                '*' if comment_start => {
                    comment_start = false;
                    in_comment_scope = true;
                }
                // Mark start of comment end if in comment scope: */
                '*' if in_comment_scope => comment_end = true,
                // Mark as comment start if not in comment scope and no comment start, mark comment_start
                '/' if !in_comment_scope => {
                    comment_start = true;
                }
                // If we get the closing delimiter, mark as non-comment scope.
                '/' if comment_end => {
                    in_comment_scope = false;
                    comment_start = false;
                    comment_end = false;
                }
                // Entering & Exiting block scope.
                '{' => in_block_scope = true,
                '}' => in_block_scope = false,
                // Any other character, reset comment start and end if not in scope.
                _ => {
                    comment_start = false;
                    comment_end = false;
                }
            }

            // No need to process this char if in bad scope.
            if in_comment_scope || in_block_scope {
                continue;
            }

            match c {
                '.' => start = Some((String::new(), false)),
                '#' => start = Some((String::new(), true)),
                _ => {}
            }
        }
    }

    (classes, ids)
}

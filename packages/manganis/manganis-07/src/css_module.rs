use crate::{AssetOptions, AssetOptionsBuilder, AssetVariant};
use const_serialize_07::SerializeConst;
use std::collections::HashSet;

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
#[doc(hidden)]
pub struct CssModuleAssetOptions {
    minify: bool,
    preload: bool,
}

impl Default for CssModuleAssetOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl CssModuleAssetOptions {
    pub const fn new() -> AssetOptionsBuilder<CssModuleAssetOptions> {
        AssetOptions::css_module()
    }

    pub const fn default() -> Self {
        Self {
            preload: false,
            minify: true,
        }
    }

    pub const fn minified(&self) -> bool {
        self.minify
    }

    pub const fn preloaded(&self) -> bool {
        self.preload
    }
}

impl AssetOptions {
    pub const fn css_module() -> AssetOptionsBuilder<CssModuleAssetOptions> {
        AssetOptionsBuilder::variant(CssModuleAssetOptions::default())
    }
}

impl AssetOptionsBuilder<CssModuleAssetOptions> {
    pub const fn with_minify(mut self, minify: bool) -> Self {
        self.variant.minify = minify;
        self
    }

    pub const fn with_preload(mut self, preload: bool) -> Self {
        self.variant.preload = preload;
        self
    }

    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: self.add_hash,
            variant: AssetVariant::CssModule(self.variant),
        }
    }
}

pub fn collect_css_idents(css: &str) -> (HashSet<String>, HashSet<String>) {
    const ALLOWED: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";

    let mut classes = HashSet::new();
    let mut ids = HashSet::new();

    let mut start: Option<(String, bool)> = None;

    let mut comment_start = false;
    let mut comment_end = false;
    let mut in_comment_scope = false;

    let mut in_block_scope = false;

    for (_byte_index, c) in css.char_indices() {
        if let Some(ident) = start.as_mut() {
            if ALLOWED.find(c).is_some() {
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
            match c {
                '*' if comment_start => {
                    comment_start = false;
                    in_comment_scope = true;
                }
                '*' if in_comment_scope => comment_end = true,
                '/' if !in_comment_scope => {
                    comment_start = true;
                }
                '/' if comment_end => {
                    in_comment_scope = false;
                    comment_start = false;
                    comment_end = false;
                }
                '{' => in_block_scope = true,
                '}' => in_block_scope = false,
                _ => {
                    comment_start = false;
                    comment_end = false;
                }
            }

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

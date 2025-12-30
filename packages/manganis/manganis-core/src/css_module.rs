use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

use crate::{AssetOptions, AssetOptionsBuilder, AssetVariant};
use const_serialize_07 as const_serialize;
use const_serialize_08::SerializeConst;

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
    const_serialize::SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[const_serialize(crate = const_serialize_08)]
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
    /// Create a new css asset using the builder
    pub const fn new() -> AssetOptionsBuilder<CssModuleAssetOptions> {
        AssetOptions::css_module()
    }

    /// Create a default css module asset options
    pub const fn default() -> Self {
        Self {
            preload: false,
            minify: true,
        }
    }

    /// Check if the asset is minified
    pub const fn minified(&self) -> bool {
        self.minify
    }

    /// Check if the asset is preloaded
    pub const fn preloaded(&self) -> bool {
        self.preload
    }
}

impl AssetOptions {
    /// Create a new css module asset builder
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/style.css", AssetOptions::css_module());
    /// ```
    pub const fn css_module() -> AssetOptionsBuilder<CssModuleAssetOptions> {
        AssetOptionsBuilder::variant(CssModuleAssetOptions::default())
    }
}

impl AssetOptionsBuilder<CssModuleAssetOptions> {
    /// Sets whether the css should be minified (default: true)
    ///
    /// Minifying the css can make your site load faster by loading less data
    pub const fn with_minify(mut self, minify: bool) -> Self {
        self.variant.minify = minify;
        self
    }

    /// Make the asset preloaded
    ///
    /// Preloading css will make the image start to load as soon as possible. This is useful for css that is used soon after the page loads or css that may not be used immediately, but should start loading sooner
    pub const fn with_preload(mut self, preload: bool) -> Self {
        self.variant.preload = preload;
        self
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: self.add_hash,
            variant: AssetVariant::CssModule(self.variant),
        }
    }
}

pub fn create_module_hash(css_path: &Path) -> String {
    let path_string = css_path.to_string_lossy();
    let mut hasher = DefaultHasher::new();
    path_string.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:016x}", hash)[..8].to_string()
}

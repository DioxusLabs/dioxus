use const_serialize::SerializeConst;

use crate::AssetOptions;

/// Options for a css asset
#[derive(
    Debug,
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

use const_serialize::SerializeConst;

use crate::AssetOptions;

/// Options for a javascript asset
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
pub struct JsAssetOptions {
    minify: bool,
    preload: bool,
}

impl Default for JsAssetOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl JsAssetOptions {
    /// Create a new js asset builder
    pub const fn new() -> Self {
        Self {
            minify: true,
            preload: false,
        }
    }

    /// Sets whether the js should be minified (default: true)
    ///
    /// Minifying the js can make your site load faster by loading less data
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, JsAssetOptions};
    /// const _: Asset = asset!("/assets/script.js", JsAssetOptions::new().with_minify(false));
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
    /// Preloading the javascript will make the javascript start to load as soon as possible. This is useful for javascript that will be used soon after the page loads or javascript that may not be used immediately, but should start loading sooner
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, JsAssetOptions};
    /// const _: Asset = asset!("/assets/script.js", JsAssetOptions::new().with_preload(true));
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
        AssetOptions::Js(self)
    }
}

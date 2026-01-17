use const_serialize_07::SerializeConst;

use crate::{AssetOptions, AssetOptionsBuilder, AssetVariant};

/// Options for a javascript asset
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
pub struct JsAssetOptions {
    minify: bool,
    preload: bool,
    static_head: bool,
}

impl Default for JsAssetOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl JsAssetOptions {
    /// Create a new js asset options builder
    pub const fn new() -> AssetOptionsBuilder<JsAssetOptions> {
        AssetOptions::js()
    }

    /// Create a default js asset options
    pub const fn default() -> Self {
        Self {
            preload: false,
            minify: true,
            static_head: false,
        }
    }

    /// Check if the asset is preloaded
    pub const fn preloaded(&self) -> bool {
        self.preload
    }

    /// Check if the asset is statically created
    pub const fn static_head(&self) -> bool {
        self.static_head
    }

    /// Check if the asset is minified
    pub const fn minified(&self) -> bool {
        self.minify
    }
}

impl AssetOptions {
    /// Create a new js asset builder
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/script.js", AssetOptions::js());
    /// ```
    pub const fn js() -> AssetOptionsBuilder<JsAssetOptions> {
        AssetOptionsBuilder::variant(JsAssetOptions::default())
    }
}

impl AssetOptionsBuilder<JsAssetOptions> {
    /// Sets whether the js should be minified (default: true)
    ///
    /// Minifying the js can make your site load faster by loading less data
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/script.js", AssetOptions::js().with_minify(false));
    /// ```
    #[allow(unused)]
    pub const fn with_minify(mut self, minify: bool) -> Self {
        self.variant.minify = minify;
        self
    }

    /// Make the asset statically inserted (default: false)
    ///
    /// Statically insert the file at compile time.
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/script.js", AssetOptions::js().with_static_head(true));
    /// ```
    #[allow(unused)]
    pub const fn with_static_head(mut self, static_head: bool) -> Self {
        self.variant.static_head = static_head;
        self
    }

    /// Make the asset preloaded
    ///
    /// Preloading the javascript will make the javascript start to load as soon as possible. This is useful for javascript that will be used soon after the page loads or javascript that may not be used immediately, but should start loading sooner
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/script.js", AssetOptions::js().with_preload(true));
    /// ```
    #[allow(unused)]
    pub const fn with_preload(mut self, preload: bool) -> Self {
        self.variant.preload = preload;
        self
    }

    /// Convert the builder into asset options with the given variant
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: self.add_hash,
            variant: AssetVariant::Js(self.variant),
        }
    }
}

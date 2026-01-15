use crate::{AssetOptions, AssetOptionsBuilder, AssetVariant};
use const_serialize_07 as const_serialize;
use const_serialize_08::SerializeConst;

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
    const_serialize::SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[const_serialize(crate = const_serialize_08)]
pub struct CssAssetOptions {
    minify: bool,
    preload: bool,
    static_head: bool,
}

impl Default for CssAssetOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl CssAssetOptions {
    /// Create a new css asset using the builder
    pub const fn new() -> AssetOptionsBuilder<CssAssetOptions> {
        AssetOptions::css()
    }

    /// Create a default css asset options
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
    /// Create a new css asset builder
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/style.css", AssetOptions::css());
    /// ```
    pub const fn css() -> AssetOptionsBuilder<CssAssetOptions> {
        AssetOptionsBuilder::variant(CssAssetOptions::default())
    }
}

impl AssetOptionsBuilder<CssAssetOptions> {
    /// Sets whether the css should be minified (default: true)
    ///
    /// Minifying the css can make your site load faster by loading less data
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/style.css", AssetOptions::css().with_minify(false));
    /// ```
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
    /// const _: Asset = asset!("/assets/style.css", AssetOptions::css().with_static_head(true));
    /// ```
    #[allow(unused)]
    pub const fn with_static_head(mut self, static_head: bool) -> Self {
        self.variant.static_head = static_head;
        self
    }

    /// Make the asset preloaded
    ///
    /// Preloading css will make the file start to load as soon as possible. This is useful for css that is used soon after the page loads or css that may not be used immediately, but should start loading sooner
    ///
    /// ```rust
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/assets/style.css", AssetOptions::css().with_preload(true));
    /// ```
    pub const fn with_preload(mut self, preload: bool) -> Self {
        self.variant.preload = preload;
        self
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: true,
            variant: AssetVariant::Css(self.variant),
        }
    }
}
